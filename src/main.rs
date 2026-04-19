
// Modules ====================================================
mod utils;
mod oled_display;
mod gyroscope;
mod servo_motor;
mod rotary_encoder;
mod stabilizer_state;

// Imports ====================================================
use std::ffi::CStr;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use std::sync::mpsc;

use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::hal::gpio::{InputMode, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;

use crate::{
    oled_display::OLEDDisplay,
    gyroscope::{Gyroscope, GyroAxis},
    servo_motor::ServoMotor,
    rotary_encoder::RotaryEncoder,
    stabilizer_state::{SharedState, StabilizerState},
    utils::{ErrorExt, StabilizerError, StabilizerResult}
};

// Constants ==================================================
const AXIS_OF_INTEREST: GyroAxis = GyroAxis::XAxis;
const THRESHOLD_STEP_SIZE: f32 = 1_f32;

// Tasks ======================================================
fn track_angle(gyro: &mut Gyroscope, tx: mpsc::Sender<f32>, shared_state: SharedState) -> StabilizerResult<()> { 
    let mut angle = 0_f32;
    let mut update;
    let mut start; let mut duration;
    let mut dt = 0_f32;
    loop {
        start = Instant::now();
        update = gyro.get_angular_velocity(AXIS_OF_INTEREST)?;
        angle += update * dt;

        tx.send(angle).map_log_error("Failed send", StabilizerError::Task("Gyro"))?;
        shared_state.set_current_angle(angle);

        duration = start.elapsed();
        dt = duration.as_secs_f32();
    }
}

fn motor_control(motor: &mut ServoMotor, rx: mpsc::Receiver<f32>, shared_state: SharedState) -> StabilizerResult<()> {
    let mut last_angle = 0_f32;
    loop {
        let abs_angle = rx.recv().map_log_error("Failed receive", StabilizerError::Task("Motor"))?;

        let (_, anchor_angle, threshold, freezed) = shared_state.snapshot();
        let rel_angle = anchor_angle - abs_angle;
        let step = (last_angle - rel_angle).abs();

        if !freezed && step > threshold {
            last_angle = rel_angle;
            motor.drive_angle(rel_angle)?;
        }
    }
}

fn threshold_control(rotary_encoder: &mut RotaryEncoder, shared_state: SharedState) -> StabilizerResult<()> {
    loop {
        let rotary_state = rotary_encoder.get_value()?;
        let threshold = (rotary_state as f32 * THRESHOLD_STEP_SIZE).max(0_f32);

        shared_state.set_threshold(threshold);

        sleep(Duration::from_millis(200));
    }
}

fn ui(display: &mut OLEDDisplay, shared_state: SharedState) -> StabilizerResult<()> {
    loop {
        let state = shared_state.snapshot();
        display.draw(state)?;

        sleep(Duration::from_secs(1));
    }
}

fn button_task<'a>(button: &mut PinDriver<'a, impl InputMode>, shared_state: SharedState) -> StabilizerResult<()> {
    block_on(async {
        loop {
            button.wait_for_falling_edge().await.map_log_error("Failed await", StabilizerError::Task("button"))?;
            let start = Instant::now();

            button.wait_for_rising_edge().await.map_log_error("Failed await", StabilizerError::Task("button"))?;
            let duration = start.elapsed().as_millis();

            if duration > 1000_u128 {
                shared_state.set_anchor();
            } else if duration > 100_u128 {
                shared_state.toggle_freeze();
            }
        }
    })
}

fn spawn_task<F>(name: &'static CStr, core: Core, task: F) -> StabilizerResult<()>
where F: FnOnce() -> StabilizerResult<()> + Send + 'static {
    let mut config = ThreadSpawnConfiguration::default();

    config.pin_to_core = Some(core);
    config.name = Some(name);
    config.set().map_log_error("Failed thread config set", StabilizerError::Other)?;

    spawn(task);
    Ok(())
}

// Main =======================================================
fn main() -> StabilizerResult<()> { 
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().map_log_error("Failed accessing peripherals", StabilizerError::Other)?;
    let pins = peripherals.pins;

    // Shared State =====================
    let state = StabilizerState::default();
    let shared_state = SharedState::new(state);
    let state_gyro = SharedState::clone(&shared_state);
    let state_ui = SharedState::clone(&shared_state);
    let state_rotary = SharedState::clone(&shared_state);
    let state_motor = SharedState::clone(&shared_state);
    let state_button = SharedState::clone(&shared_state);

    // Channels =========================
    let (tx_gtm, rx_gtm) = mpsc::channel::<f32>();
    
    // OLED Display Component ===========
    let mut display_wrapper = OLEDDisplay::new(
        peripherals.i2c0, 
        pins.gpio17,
        pins.gpio18,
        pins.gpio21,
        pins.gpio36
    )?;
    display_wrapper.test_draw()?;
    let mut boxed_display = Box::new(display_wrapper);

    // Gyroscope Component ==============
    let mut gyro = Gyroscope::new(
        peripherals.i2c1,
        pins.gpio3.into(),
        pins.gpio2.into()
    )?;
    gyro.calibrate(AXIS_OF_INTEREST)?; 
    let mut boxed_gyro = Box::new(gyro);

    // Servo Motor Component ============
    let motor = ServoMotor::new(
        pins.gpio7.into(),
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
    )?;
    let mut boxed_motor = Box::new(motor);
   
    // RotaryEncode Component ===========
    let rotary_encoder = RotaryEncoder::new(
        pins.gpio33.into(),
        pins.gpio34.into(),
    )?;
    let mut boxed_rotary = Box::new(rotary_encoder);

    // Button Press =====================
    let mut axial_button = PinDriver::input(pins.gpio35, esp_idf_svc::hal::gpio::Pull::Down)
        .map_log_error("Failed button creation", StabilizerError::Other)?;

    // Spawn Tasks ======================
    log::info!("Ready to spawn tasks");
    
    // Core 0
    spawn_task(c"UI_task", Core::Core0, move || {
        ui(&mut boxed_display, state_ui)
    })?; 
    log::info!("Spawned UI task");

    spawn_task(c"threshold_task", Core::Core0, move || {
        threshold_control(&mut boxed_rotary, state_rotary)
    })?;
    log::info!("Spawned threshold control task");

    spawn_task(c"button_task", Core::Core0, move || {
        button_task(&mut axial_button, state_button)
    })?;
    log::info!("Spawned button task");

    // Core 1
    spawn_task(c"motor_task", Core::Core1, move || {
        motor_control(&mut boxed_motor, rx_gtm, state_motor)
    })?; 
    log::info!("Spawned motor control task");
 
    spawn_task(c"gyro_task", Core::Core1, move || {
        track_angle(&mut boxed_gyro, tx_gtm, state_gyro)
    })?;
    log::info!("Spawned gyro task");

    // Display debug info on serial monitor
    loop {
        sleep(Duration::from_millis(500));

        let (current_angle, _anchor_angle, threshold, _freezed) = shared_state.snapshot();

        log::info!("Current angle: {:}", current_angle);
        log::info!("Threshold: {:}", threshold);
    }
}
