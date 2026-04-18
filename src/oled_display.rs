
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
    mono_font::{MonoTextStyleBuilder, iso_8859_1::FONT_7X13}, 
    pixelcolor::BinaryColor, 
    prelude::*, 
    primitives::{Circle, Line, PrimitiveStyleBuilder, Sector}, 
    text::Text
};

use crate::utils::ErrorExt;

const DISPLAY_WIDTH: i32 = 128;
const DISPLAY_HEIGHT: i32 = 64;

#[derive(Debug)]
pub enum OLEDError {
    Draw,
    DriverInit,
    Flush,
    I2CBus,
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
        -> Result<OLEDDisplay<'a>, OLEDError> {

        // Drive vext low to enable the OLED display
        let mut vext_driver = PinDriver::output(vext)
            .map_log_error("Failed vext pin driver init", OLEDError::DriverInit)?;
        vext_driver.set_low().map_log_error("Failed vext pin driver set low", OLEDError::DriverInit)?;
     
        // Reset sequence to wake up the display
        let mut reset = PinDriver::output(rst)
            .map_log_error("Failed reset pin driver init", OLEDError::DriverInit)?;
        reset.set_low().map_log_error("Failed reset pin driver set low", OLEDError::DriverInit)?;
        sleep(Duration::from_millis(50));
        reset.set_high().map_log_error("Failed reset pin driver set high", OLEDError::DriverInit)?;

        // Init I2C bus
        let config = I2cConfig::new().baudrate(esp_idf_svc::hal::units::Hertz(100000));
        let i2c = I2cDriver::new(
            i2c_bus,
            sda,
            scl,
            &config,
        ).map_log_error("Failed i2c bus init", OLEDError::I2CBus)?;

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

    fn draw_circle(self: &mut Self, angle: f32, anchor_angle: f32) -> Result<(), OLEDError> {
        
        let left_corner = Point::new(2 + DISPLAY_WIDTH / 2, 2);
        let diameter = (DISPLAY_HEIGHT - 4) as u32;

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
            .map_log_error("Failed to fill sector in buffer", OLEDError::Draw)?;

        let style_outer = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(2)
            .build();

        Circle::new(left_corner, diameter)
            .into_styled(style_outer)
            .draw(&mut self.display)
            .map_log_error("Failed to draw outer circle to buffer", OLEDError::Draw)?;

        let radius = (2 + DISPLAY_HEIGHT / 2) as f32;
        let dir_angle = (anchor_angle - 90_f32).to_radians();
        let center = Point::new(DISPLAY_WIDTH * 3 / 4, DISPLAY_HEIGHT / 2);
        let delta = Point::new((radius * dir_angle.cos()) as i32, (radius * dir_angle.sin()) as i32);
        let line_style = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(2)
            .build();
        Line::with_delta(center, delta)
            .into_styled(line_style)
            .draw(&mut self.display)
            .map_log_error("Failed to draw line to buffer", OLEDError::Draw)?;

        Ok(())
    }

    fn draw_information(self: &mut Self, angle: f32, threshold: f32) -> Result<(), OLEDError> {
 
        let style = MonoTextStyleBuilder::new()
            .text_color(BinaryColor::On)
            .font(&FONT_7X13)
            .build(); 

        Text::new("Angle:", Point::new(2, 10), style)
            .draw(&mut self.display)
            .map_log_error("Failed to write second text line", OLEDError::Draw)?;

        let degree_str = format!("{:.2}°", angle);

        Text::new(&degree_str, Point::new(2, 25), style)
            .draw(&mut self.display)
            .map_log_error("Failed to write angle text line", OLEDError::Draw)?;

        Text::new("Thresh.:", Point::new(2, 40), style)
            .draw(&mut self.display)
            .map_log_error("Failed to write third text line", OLEDError::Draw)?;

        let threshold_str = format!("{:.0}°", threshold);
        Text::new(&threshold_str, Point::new(2, 55), style)
            .draw(&mut self.display)
            .map_log_error("Failed to write threshold text line", OLEDError::Draw)?;

        Ok(())
    }

    pub fn draw(self: &mut Self, state: (f32, f32, f32, bool)) -> Result<(), OLEDError> {

        let (current_angle, anchor_angle, threshold, freezed) = state;

        self.display.clear_buffer();

        self.draw_information(current_angle, threshold)?;

        self.draw_circle(current_angle, anchor_angle)?;

        self.display.set_invert(freezed).map_log_error("Failed screen inversion", OLEDError::Draw)?;
        
        self.display.flush().map_log_error("Failed to flush during draw", OLEDError::Flush)?;

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
