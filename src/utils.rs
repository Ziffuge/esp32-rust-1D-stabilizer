
use crate::{
    gyroscope::GyroError, 
    oled_display::OLEDError, 
    rotary_encoder::RotaryEncoderError, 
    servo_motor::ServoMotorError
};


pub trait ErrorExt<T, E, OE> {
    fn map_log_error(self, context: &str, suberror: E) -> Result<T, E>;
}

impl <T, E, OE> ErrorExt<T, E, OE> for Result<T, OE>
where OE: std::fmt::Debug {
    fn map_log_error(self, context: &str, suberror: E) -> Result<T, E> {
        self.map_err(|e| {
            log::error!("{:} : {:?}", context, e);
            suberror
        })
    }
}

pub type StabilizerResult<T> = Result<T, StabilizerError>;

#[derive(Debug)]
pub enum StabilizerError {
    Gyro(GyroError),
    OLEDDisplay(OLEDError),
    Other,
    RotaryEncoder(RotaryEncoderError),
    ServoMotor(ServoMotorError),
    Task(&'static str),
}

impl From<GyroError> for StabilizerError {
    fn from(err: GyroError) -> Self {
        StabilizerError::Gyro(err)
    }
}

impl From<OLEDError> for StabilizerError {
    fn from(err: OLEDError) -> Self {
        StabilizerError::OLEDDisplay(err)
    }
}

impl From<RotaryEncoderError> for StabilizerError {
    fn from(err: RotaryEncoderError) -> Self {
        StabilizerError::RotaryEncoder(err)
    }
}

impl From<ServoMotorError> for StabilizerError {
    fn from(err: ServoMotorError) -> Self {
        StabilizerError::ServoMotor(err)
    }
}


