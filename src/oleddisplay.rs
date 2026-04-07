
use std::thread::sleep;
use std::time::Duration;
use std::error::Error;

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
    mono_font::{MonoTextStyleBuilder, iso_8859_1::FONT_7X13}, 
    pixelcolor::BinaryColor, 
    prelude::*, 
    primitives::{Circle, PrimitiveStyleBuilder, Sector}, 
    text::Text
};

pub enum OLEDError {
    Draw,
    Flush
}

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
        -> Result<OLEDDisplay<'a>, Box<dyn Error>> {

        // Drive vext low to enable the OLED display
        let mut vext_driver = PinDriver::output(vext)?;
        vext_driver.set_low()?;
     
        // Reset sequence to wake up the display
        let mut reset = PinDriver::output(rst)?;
        reset.set_low()?;
        sleep(Duration::from_millis(50));
        reset.set_high()?;

        // Init I2C bus
        let config = I2cConfig::new().baudrate(esp_idf_svc::hal::units::Hertz(100000));
        let i2c = I2cDriver::new(
            i2c_bus,
            sda,
            scl,
            &config,
        )?;

        // Init display driver
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
    
        let init_r = display.init();

        if let Err(_e) = init_r {
            log::error!("Display init failed");
        }

        Ok(OLEDDisplay { 
            display: display,
            _reset: reset, 
            _vext: vext_driver
        })
    }

    fn draw_circle(self: &mut Self, angle: f32) -> Result<(), OLEDError> {
        let display_width = 128;
        let display_height = 64; 

        let left_corner = Point::new(2 + display_width / 2, 2);
        let diameter = display_height - 4;

        let style_inner = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(2)
            .fill_color(BinaryColor::On)
            .build(); 

        let start_angle = Angle::from_degrees(0_f32 + angle);
        let end_angle = Angle::from_degrees(180_f32);
        Sector::new(left_corner, diameter, start_angle, end_angle)
            .into_styled(style_inner)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to fill sector in buffer: {:?}", e);
                OLEDError::Draw
            })?;

        let style_outer = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(2)
            .build();

        Circle::new(left_corner, diameter)
            .into_styled(style_outer)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to draw outer circle to buffer: {:?}", e);
                OLEDError::Draw
            })?;


        Ok(())
    }

    fn draw_information(self: &mut Self, angle: f32) -> Result<(), OLEDError> {
 
        let style = MonoTextStyleBuilder::new()
            .text_color(BinaryColor::On)
            .font(&FONT_7X13)
            .build();

        Text::new("Current", Point::new(2, 10), style)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to write first text line: {:?}", e);
                OLEDError::Draw
            })?;

        Text::new("angle:", Point::new(2, 25), style)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to write second text line: {:?}", e);
                OLEDError::Draw
            })?;

        let degree_str = format!("{:.2}°", angle);

        Text::new(&degree_str, Point::new(2, 40), style)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to write angle text line: {:?}", e);
                OLEDError::Draw
            })?;

        Ok(())
    }

    pub fn draw(self: &mut Self, angle: f32) -> Result<(), OLEDError> {

        self.display.clear_buffer();

        self.draw_information(angle)?;

        self.draw_circle(angle)?;
        
        self.display.flush().map_err(|e| {
            log::error!("Failed to flush during draw: {:?}", e);
            OLEDError::Flush
        })?;

        Ok(())
    }

    pub fn test_draw(self: &mut Self) -> Result<(), OLEDError> {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_7X13)
            .text_color(BinaryColor::On)
            .build();

        Text::new("Test Draw", Point::new(0, 10), text_style)
            .draw(&mut self.display)
            .map_err(|e| {
                log::error!("Failed to draw during test_draw: {:?}", e);
                OLEDError::Draw
            })?; 

        self.display.flush().map_err(|e| {
            log::error!("Failed to flush during test_draw: {:?}", e);
            OLEDError::Flush
        })?;

        Ok(())
    }
}
