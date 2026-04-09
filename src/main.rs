
// Modules
mod oleddisplay;
mod gyroscope;
mod servo_motor;

// Imports
use std::thread::sleep;
use std::time::Duration;

use esp_idf_svc::hal::peripherals::Peripherals;

use oleddisplay::OLEDDisplay;
use gyroscope::{Gyroscope, GyroAxis};
use servo_motor::ServoMotor;

fn main() -> Result<(), Box<dyn std::error::Error>> { 
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let pins = peripherals.pins;
    
    let mut display_wrapper = OLEDDisplay::new(
        peripherals.i2c0, 
        pins.gpio17,
        pins.gpio18,
        pins.gpio21,
        pins.gpio36
    )?;

    display_wrapper.test_draw();

    let mut gyro = Gyroscope::new(
        peripherals.i2c1,
        pins.gpio3.into(),
        pins.gpio2.into()
    ).map_err(|_| format!("Failed gyro creation"))?;

    gyro.calibrate(GyroAxis::XAxis).map_err(|_| format!("Failed gyro calibration"))?;

    let mut motor = ServoMotor::new(
        pins.gpio7.into(),
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
        ).map_err(|_| format!("Failed motor creation"))?;
    
    let mut angle = 0_f32;
    let mut gyro_output;
    // Keep the program alive
    loop {
        sleep(Duration::from_millis(500));
        display_wrapper.draw(angle);

        gyro_output = gyro.get_angular_velocity(GyroAxis::XAxis)
            .map_err(|_| format!("Failed gyro read"))?;

        angle += gyro_output * 0.5_f32;
        angle = angle % 360_f32;

        motor.drive_angle(-angle)
            .map_err(|_| format!("Failed motor drive"))?;

        log::info!("Gyro Output: {:}", gyro_output);
    }
}
