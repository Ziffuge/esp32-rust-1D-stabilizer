
use std::time::Duration;

use esp_idf_svc::hal::{
    gpio::{AnyInputPin}, 
    pcnt::{config::{ChannelConfig, GlitchFilterConfig, UnitConfig}, *}
};

use crate::utils::ErrorExt; 

#[derive(Debug)]
pub enum RotaryEncoderError {
    ChannelConfig,
    DriverInit,
    GetCount,
    GlitchFilter,
}

pub struct RotaryEncoder<'a> {
    unit: PcntUnitDriver<'a>,
}

const LOW_LIMIT: i32 = -1_i32;
const HIGH_LIMIT: i32 = 361_i32; 

impl <'a> RotaryEncoder<'a> {

    pub fn new(pina: AnyInputPin<'a>, pinb: AnyInputPin<'a>) -> Result<Self, RotaryEncoderError> {

        // Set up shaft rotation ========
        let config = UnitConfig { 
            low_limit: LOW_LIMIT,
            high_limit: HIGH_LIMIT,
            intr_priority: 0,
            accum_count: true,
            __internal: ()
        };
        let mut unit = PcntUnitDriver::new(&config)
            .map_log_error("Failed pcnt unit initialization", RotaryEncoderError::DriverInit)?;
        
        let glitchfilterconfig = GlitchFilterConfig {
            max_glitch: Duration::from_micros(10),
            __internal: ()
        };
        unit.set_glitch_filter(Some(&glitchfilterconfig))
            .map_log_error("Failed setting glitch filter", RotaryEncoderError::GlitchFilter)?; 

        // Clockwise channel : A rises before B
        let channelconfig = ChannelConfig::default();
        unit.add_channel(Some(pina), Some(pinb), &channelconfig)
            .map_log_error("Failed channel config (add)", RotaryEncoderError::ChannelConfig)?
            .set_edge_action(
                config::ChannelEdgeAction::Increase,
                config::ChannelEdgeAction::Decrease
            ).map_log_error("Failed channel config (edge action)", RotaryEncoderError::ChannelConfig)?
            .set_level_action(
                config::ChannelLevelAction::Inverse,
                config::ChannelLevelAction::Keep
            ).map_log_error("Failed channel config (level action)", RotaryEncoderError::ChannelConfig)?; 

        unit.enable().map_log_error("Failed pcnt unit enable", RotaryEncoderError::DriverInit)?;

        unit.add_watch_points_and_clear([LOW_LIMIT, HIGH_LIMIT])
            .map_log_error("Failed pcnt unit watch points", RotaryEncoderError::DriverInit)?;

        unit.start().map_log_error("Failed pcnt unit start", RotaryEncoderError::DriverInit)?;

        Ok(RotaryEncoder { 
            unit: unit,
        })
    }

    pub fn get_value(self: &mut Self) -> Result<i32, RotaryEncoderError> {
        self.unit.get_count().map_log_error("Failed to get count", RotaryEncoderError::GetCount)
    }
}
