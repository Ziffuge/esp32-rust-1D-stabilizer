
// Modules ====================================================
mod utils;
mod oled_display;
mod gyroscope;
mod servo_motor;
mod rotary_encoder;
mod stabilizer_state;

// Imports ====================================================
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use std::sync::{mpsc, Arc, Mutex};

use esp_idf_svc::hal::gpio::{InputMode, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;

use oled_display::{OLEDDisplay, OLEDError};
use gyroscope::{Gyroscope, GyroAxis, GyroError};
use servo_motor::{ServoMotor, ServoMotorError};
use rotary_encoder::{RotaryEncoder, RotaryEncoderError};
use stabilizer_state::StabilizerState;

// Constants ==================================================
const AXIS_OF_INTEREST: GyroAxis = GyroAxis::XAxis;
const THRESHOLD_STEP_SIZE: f32 = 1_f32;

// Tasks ======================================================
fn track_angle(gyro: &mut Gyroscope, tx: mpsc::Sender<f32>, shared_state: Arc<Mutex<StabilizerState>>) -> Result<(), GyroError> { 
    let mut angle = 0_f32;
    let mut update;
    let mut start; let mut duration;
    let mut dt = 0_f32;
    loop {
        start = Instant::now();
        update = gyro.get_angular_velocity(AXIS_OF_INTEREST)?;
        angle += update * dt;

        tx.send(angle).unwrap();
        shared_state.lock().unwrap().current_angle = angle;

        duration = start.elapsed();
        dt = duration.as_secs_f32();
    }
}

fn motor_control(motor: &mut ServoMotor, rx: mpsc::Receiver<f32>, shared_state: Arc<Mutex<StabilizerState>>) -> Result<(), ServoMotorError> {
    loop {
        let abs_angle = rx.recv().unwrap();
        let StabilizerState { current_angle: _, anchor_angle ,threshold, freezed } = *shared_state.lock().unwrap();
        let rel_angle = anchor_angle - abs_angle;
        if !freezed && rel_angle.abs() > threshold {
            motor.drive_angle(rel_angle)?;
        }
    }
}

fn threshold_control(rotary_encoder: &mut RotaryEncoder, shared_state: Arc<Mutex<StabilizerState>>) -> Result<(), RotaryEncoderError> {
    loop {
        let rotary_state = rotary_encoder.get_value()?;
        let threshold = (rotary_state as f32 * THRESHOLD_STEP_SIZE).max(0_f32);

        shared_state.lock().unwrap().threshold = threshold;

        sleep(Duration::from_millis(500));
    }
}

fn ui(display: &mut OLEDDisplay, shared_state: Arc<Mutex<StabilizerState>>) -> Result<(), OLEDError> {
    loop {
        let state_snapshot = shared_state.lock().unwrap().clone();
        display.draw(state_snapshot)?;

        sleep(Duration::from_secs(1));
    }
}

fn button_task<'a>(button: &mut PinDriver<'a, impl InputMode>, shared_state: Arc<Mutex<StabilizerState>>) {
    block_on(async {
        loop {
            button.wait_for_falling_edge().await;
            let start = Instant::now();

            button.wait_for_rising_edge().await;
            let duration = start.elapsed().as_millis();

            if duration > 1000_u128 {
                shared_state.lock().unwrap().set_anchor();
            } else if duration > 100_u128 {
                shared_state.lock().unwrap().toggle_freeze();
            }
        }
    });
}

// Main =======================================================
fn main() -> Result<(), Box<dyn std::error::Error>> { 
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;

    // Shared State =====================
    let state = StabilizerState::default();
    let shared_state = Arc::new(Mutex::new(state));
    let state_gyro = Arc::clone(&shared_state);
    let state_ui = Arc::clone(&shared_state);
    let state_rotary = Arc::clone(&shared_state);
    let state_motor = Arc::clone(&shared_state);
    let state_button = Arc::clone(&shared_state);

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
    display_wrapper.test_draw().map_err(|_| format!("Failed OLED display test"))?;
    let mut boxed_display = Box::new(display_wrapper);

    // Gyroscope Component ==============
    let mut gyro = Gyroscope::new(
        peripherals.i2c1,
        pins.gpio3.into(),
        pins.gpio2.into()
    ).map_err(|_| format!("Failed gyro creation"))?;
    gyro.calibrate(AXIS_OF_INTEREST).map_err(|_| format!("Failed gyro calibration"))?; 
    let mut boxed_gyro = Box::new(gyro);

    // Servo Motor Component ============
    let motor = ServoMotor::new(
        pins.gpio7.into(),
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
    ).map_err(|_| format!("Failed motor creation"))?;
    let mut boxed_motor = Box::new(motor);
   
    // RotaryEncode Component ===========
    let rotary_encoder = RotaryEncoder::new(
        pins.gpio33.into(),
        pins.gpio34.into(),
        //pins.gpio35.into(),
    ).map_err(|_| format!("Failed rotary creation"))?;
    let mut boxed_rotary = Box::new(rotary_encoder);

    // Button Press =====================
    let mut axial_button = PinDriver::input(pins.gpio35, esp_idf_svc::hal::gpio::Pull::Down)
        .map_err(|_| format!("Failed button creation"))?; 

    // Spawn Tasks ======================
    log::info!("Ready to spawn tasks");
    let mut thread_config = ThreadSpawnConfiguration::default();

    thread_config.pin_to_core = Some(esp_idf_svc::hal::cpu::Core::Core0);
    thread_config.set()?;

    thread_config.name = Some(c"UI_task");
    thread_config.set()?;
    spawn(move || {
        ui(&mut boxed_display, state_ui);
    });
    log::info!("Spawned UI task");

    spawn(move || {
        threshold_control(&mut boxed_rotary, state_rotary);
    });
    log::info!("Spawned threshold control task");

    spawn(move || {
        button_task(&mut axial_button, state_button);
    });
    log::info!("Spawned button task");

    thread_config.pin_to_core = Some(esp_idf_svc::hal::cpu::Core::Core1);
    thread_config.set()?;

    thread_config.name = Some(c"motor_task");
    thread_config.set()?;
    spawn(move || {
        motor_control(&mut boxed_motor, rx_gtm, state_motor);
    });
    log::info!("Spawned motor control task");

    thread_config.name = Some(c"gyro_task");
    thread_config.set()?;
    spawn(move || {
        track_angle(&mut boxed_gyro, tx_gtm, state_gyro);
    });
    log::info!("Spawned gyro task");

    // Display debug info on serial monitor
    loop {
        sleep(Duration::from_millis(500));

        let StabilizerState { current_angle, anchor_angle:_ , threshold, freezed:_ } = *shared_state.lock().unwrap();

        log::info!("Current angle: {:}", current_angle);
        log::info!("Threshold: {:}", threshold);
    }
}
