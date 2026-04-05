
// Modules
mod oleddisplay;

// Imports
use std::thread::sleep;
use std::time::Duration;

use esp_idf_svc::hal::peripherals::Peripherals;

use oleddisplay::OLEDDisplay;

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
    
    let mut angle = 0_f32;
    // Keep the program alive
    loop {
        sleep(Duration::from_secs(1));
        display_wrapper.draw(angle);
        angle += 1_f32;
        angle = angle % 360_f32;
    }
}
