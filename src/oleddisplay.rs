
use std::thread::sleep;
use std::time::Duration;

use esp_idf_svc::hal::{
    gpio::{InputPin, OutputPin, PinDriver, Output}, 
    i2c::{I2c, I2cConfig, I2cDriver}
};

use ssd1306::{
    Ssd1306,
    I2CDisplayInterface,
    mode::{DisplayConfig, BufferedGraphicsMode},
    prelude::{DisplayRotation, DisplaySize128x64, I2CInterface}
};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

pub type DI<'a> = I2CInterface<I2cDriver<'a>>;
pub type SIZE = DisplaySize128x64;
pub type MODE = BufferedGraphicsMode<SIZE>;
pub struct OLEDDisplay<'a> {
     pub display: Ssd1306<DI<'a>, SIZE, MODE>,
     _reset: PinDriver<'a, Output>,
     _vext: PinDriver<'a, Output>
}

impl <'a> OLEDDisplay<'a> {
    pub fn new<
        T: I2c + 'a,
        U:InputPin + OutputPin + 'a, 
        V:InputPin + OutputPin + 'a,
        W:InputPin + OutputPin + 'a, 
        X:InputPin + OutputPin + 'a> (i2c_bus: T, sda: U, scl: V, rst: W, vext: X) 
        -> Result<OLEDDisplay<'a>, Box<dyn std::error::Error>> {
      
        log::info!("OLEDDisplay::new begin");

        // Drive vext low to enable the OLED display
        let mut vext_driver = PinDriver::output(vext)?;
        vext_driver.set_low()?;

        log::info!("OLEDDisplay::new vext");
     
        // Reset sequence to wake up the display
        let mut reset = PinDriver::output(rst)?;
        reset.set_low()?;
        sleep(Duration::from_millis(50));
        reset.set_high()?;

        log::info!("OLEDDisplay::new reset");

        // Init I2C bus
        let config = I2cConfig::new().baudrate(esp_idf_svc::hal::units::Hertz(100000));
        let i2c = I2cDriver::new(
            i2c_bus,
            sda, // SDA
            scl, // SCL
            &config,
        )?;

        // Init display driver
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
    
        let init_r = display.init();

        if let Err(e) = init_r {
            log::error!("Display init failed");
        }

        log::info!("OLEDDisplay::new end");
        Ok(OLEDDisplay { 
            display: display,
            _reset: reset, 
            _vext: vext_driver
        })
    }

    pub fn draw(self: &mut Self) {
        log::info!("About to draw");
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        Text::new("Heltec LoRa V3", Point::new(0, 10), text_style)
            .draw(&mut self.display).unwrap(); 

        Text::new("Rust is Running!", Point::new(0, 30), text_style)
            .draw(&mut self.display).unwrap();

        log::info!("About to flush");
        self.display.flush().unwrap(); 
    }
}
