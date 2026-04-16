
use esp_idf_svc::hal::{
    gpio::AnyIOPin, 
    i2c::{I2c, I2cConfig, I2cDriver}, 
    units::Hertz
};

use crate::utils::ErrorExt;

// See https://www.invensense.com/wp-content/uploads/2015/02/MPU-6000-Datasheet1.pdf
//     section 9.2
static MPU_ADDRESS: u8 = 0b1101000;

#[derive(Clone, Copy)]
pub enum GyroAxis {
    XAxis,
    YAxis,
    ZAxis
}

#[repr(u8)]
enum GyroRegister {
    GyroXOutHigh = 0x43_u8,
    GyroXOutLow = 0x44_u8,
    
    GyroYOutHigh = 0x45_u8,
    GyroYOutLow = 0x46_u8,

    GyroZOutHigh = 0x47_u8,
    GyroZOutLow = 0x48_u8,

    GyroConfig = 0x1B_u8,

    FIFOEnable = 0x23_u8,
    UserCtrl = 0x6A_u8,
    PowerMgmt1 = 0x6B_u8,
    PowerMgmt2 = 0x6C_u8,
}

#[derive(Debug)]
pub enum GyroError {
    I2CDriver,
    RegRead,
    RegWrite
}

#[repr(u8)]
pub enum GyroFSRange {
    PM250 = 0b000_00_000_u8,
    PM500 = 0b000_01_000_u8,
    PM1000 = 0b000_10_000_u8,
    PM2000 = 0b000_11_000_u8
}

pub struct Gyroscope<'a> {
    i2c_driver: I2cDriver<'a>,
    _current_range: GyroFSRange,
    _bias_x: i16,
    _bias_y: i16,
    _bias_z: i16,
}

impl <'a> Gyroscope<'a> {
    pub fn new(i2c_bus: impl I2c + 'a, sda: AnyIOPin<'a>, scl: AnyIOPin<'a>) -> Result<Self, GyroError> {
        
        let config = I2cConfig::new().baudrate(Hertz::from(400000));
        let mut i2c = I2cDriver::new(i2c_bus, sda, scl, &config)
            .map_log_error("Failed to init i2c driver for gyroscope", GyroError::I2CDriver)?;

        // Wake up from sleep  
        let mut power_mgmt_reg = [0_u8];
        Self::read_register(&mut i2c, GyroRegister::PowerMgmt1, &mut power_mgmt_reg)?;

        let new_reg_value = power_mgmt_reg[0] & 0b10111111;
        Self::write_register(&mut i2c, GyroRegister::PowerMgmt1, new_reg_value)?; 

        Ok(Gyroscope {
            i2c_driver: i2c,
            _current_range: GyroFSRange::PM250,
            _bias_x: 0_i16,
            _bias_y: 0_i16,
            _bias_z: 0_i16,
        })
    }

    pub fn calibrate(self: &mut Self, axis: GyroAxis) -> Result<(), GyroError> {
        
        let mut acc = 0_i64;
        let n = 100;

        for _ in 0..n {
            acc += self.get_raw_angular_velocity(axis)? as i64;
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        match axis {
            GyroAxis::XAxis => self._bias_x = (acc / n) as i16,
            GyroAxis::YAxis => self._bias_y = (acc / n) as i16,
            GyroAxis::ZAxis => self._bias_z = (acc / n) as i16,
        }

        Ok(())
    }

    pub fn get_angular_velocity(self: &mut Self, axis: GyroAxis) -> Result<f32, GyroError> {

        let lsb_angular_velocity = self.get_raw_angular_velocity(axis)?;

        let lsb_sensitivity = match self._current_range {
            GyroFSRange::PM250 => 131_f32,
            GyroFSRange::PM500 => 65.5_f32,
            GyroFSRange::PM1000 => 32.8_f32,
            GyroFSRange::PM2000 => 16.4_f32,
        };

        let bias = match axis {
            GyroAxis::XAxis => self._bias_x,
            GyroAxis::YAxis => self._bias_y,
            GyroAxis::ZAxis => self._bias_z,
        };

        let angular_velocity = (lsb_angular_velocity - bias) as f32 / lsb_sensitivity;

        Ok(angular_velocity)
    }

    pub fn set_range(self: &mut Self, range: GyroFSRange) -> Result<(), GyroError> {
        let mut gyroc_reg = [0_u8];
        Self::read_register(&mut self.i2c_driver, GyroRegister::GyroConfig, &mut gyroc_reg)?;
        
        let new_reg_value = gyroc_reg[0] | range as u8;
        Self::write_register(&mut self.i2c_driver, GyroRegister::GyroConfig, new_reg_value)?;

        Ok(())
    }

    fn get_raw_angular_velocity(self: &mut Self, axis: GyroAxis) -> Result<i16, GyroError> {
        let mut buffer = [0_u8; 2];
        let (left, right) = buffer.split_at_mut(1);

        let (register_high, register_low) = match axis {
            GyroAxis::XAxis => (GyroRegister::GyroXOutHigh, GyroRegister::GyroXOutLow),
            GyroAxis::YAxis => (GyroRegister::GyroYOutHigh, GyroRegister::GyroYOutLow),
            GyroAxis::ZAxis => (GyroRegister::GyroZOutHigh, GyroRegister::GyroZOutLow),
        }; 

        Self::read_register(&mut self.i2c_driver, register_high, left)?;

        Self::read_register(&mut self.i2c_driver, register_low, right)?;
        
        Ok(i16::from_be_bytes(buffer))
    }

    fn read_register(i2c_driver: &mut I2cDriver, register: GyroRegister, buffer: &mut [u8]) -> Result<(), GyroError> {

        i2c_driver.write_read(MPU_ADDRESS, &[register as u8], buffer, 1000)
            .map_log_error("Failed generic register read", GyroError::RegRead)?;

        Ok(())
    }

    fn write_register(i2c_driver: &mut I2cDriver, register: GyroRegister, data: u8) -> Result<(), GyroError> {

        let command = [register as u8, data];
        i2c_driver.write(MPU_ADDRESS, &command, 1000)
            .map_log_error("Failed generic register write", GyroError::RegRead)?;

        Ok(())
    }
}
