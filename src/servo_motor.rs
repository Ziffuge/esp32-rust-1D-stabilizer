
// Use LED controller as a general purpose 
// Pulse Width Modulator (PWM) controller
use esp_idf_svc::hal::{
    gpio::AnyIOPin, 
    ledc::{
        LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, LowSpeed,
        config::{Resolution, TimerConfig}
    },
    units::Hertz
};

pub type MotorDriver<'a> = LedcDriver<'a>;

pub enum ServoMotorError {
    AngleOverflow,
    DriverInit,
    MotorDrive,
}

pub struct ServoMotor<'a> {
    motordriver: MotorDriver<'a>,
}

const MAX_DUTY_PERCENTAGE: f32 = 12.4_f32;
const MIN_DUTY_PERCENTAGE: f32 = 2.6_f32;
const MAX_ANGLE: f32 = 90_f32;
const MIN_ANGLE: f32 = -90_f32;

impl <'a> ServoMotor<'a> {

    pub fn new(
        pwmpin: AnyIOPin<'a>, 
        timer: impl LedcTimer<SpeedMode = LowSpeed> + 'a, 
        channel: impl LedcChannel<SpeedMode = LowSpeed> + 'a
    ) -> Result<Self, ServoMotorError> {
 
        let config = TimerConfig::default()
            .frequency(Hertz::from(50))
            .resolution(Resolution::Bits12);
         
        let timerdriver = LedcTimerDriver::new(
            timer,
            &config
        ).map_err(|e| {
            log::error!("Failed timer driver init: {:?}", e);
            ServoMotorError::DriverInit
        })?;

        let mut driver = LedcDriver::new(
            channel,
            timerdriver,
            pwmpin,
        ).map_err(|e| {
            log::error!("Failed pwm driver initialization: {:?}", e);
            ServoMotorError::DriverInit
        })?; 

        Self::test_range(&mut driver)?;
        Self::reset_motor(&mut driver)?;

        Ok(ServoMotor { 
            motordriver: driver, 
        })
    }

    pub fn drive(self: &mut Self, percentage: f32) -> Result<(), ServoMotorError> { 
        Self::drive_motor(&mut self.motordriver, percentage)
    }

    pub fn drive_angle(self: &mut Self, angle: f32) -> Result<(), ServoMotorError> {
        let percentage = Self::angle_to_percentage(angle)?; 
        self.drive(percentage)
    }

    fn drive_motor(motor: &mut MotorDriver, percentage: f32) -> Result<(), ServoMotorError> {
        
        let duty = (percentage / 100_f32) * motor.get_max_duty() as f32;
        motor.set_duty(duty.round() as u32)
            .map_err(|e| {
                log::error!("Failed to drive motor: {:?}", e);
                ServoMotorError::MotorDrive
            })?;

        Ok(())

    }

    fn reset_motor(motor: &mut MotorDriver) -> Result<(), ServoMotorError> {
        Self::drive_motor(motor, 7.5_f32)
    }

    fn angle_to_percentage(angle: f32) -> Result<f32, ServoMotorError> {
        if angle < MIN_ANGLE || angle > MAX_ANGLE {
            log::error!("Angle overflow: {:.2}", angle);
            return Err(ServoMotorError::AngleOverflow);
        }

        let mut result = angle;

        // Map to [0.0, 1.0]
        result = (result + MAX_ANGLE) / (MAX_ANGLE - MIN_ANGLE);

        // Move to range [2.6, 12.4]
        result = (result * (MAX_DUTY_PERCENTAGE - MIN_DUTY_PERCENTAGE)) + MIN_DUTY_PERCENTAGE;

        Ok(result)
    }

    fn test_range(motor: &mut MotorDriver) -> Result<(), ServoMotorError> {
        log::info!("Going full left");
        Self::drive_motor(motor, MIN_DUTY_PERCENTAGE)?;
        std::thread::sleep(std::time::Duration::from_secs(3));

        log::info!("Going full right");
        Self::drive_motor(motor, MAX_DUTY_PERCENTAGE)?;
        std::thread::sleep(std::time::Duration::from_secs(3));
        Ok(())
    }
}
