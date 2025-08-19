use defmt::*;
use embassy_stm32::adc::{Adc, SampleTime};
// use embassy_stm32::dac::{Dac, DacChannel};
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::{Peri, Peripherals, peripherals::*};

/// Concrete HAL for STM32G474RE
pub struct Hal {
    pub adc1: Adc<'static, ADC1>,
    pub adc2: Adc<'static, ADC2>,
    // pub dac1: Dac<'static, DAC1>,
    // pub dac2: Dac<'static, DAC2>,
    pub led: Output<'static>,
    pub adc_channels: AdcChannels,
    pub button: Input<'static>,
}

pub struct AdcChannels {
    pub channel_regulator_actual_pressure: Peri<'static, PA0>,
    pub channel_systemic_flow: Peri<'static, PA1>,
    pub channel_pulmonary_flow: Peri<'static, PA2>,
    pub channel_systemic_preload_pressure: Peri<'static, PA3>,
    pub channel_systemic_afterload_pressure: Peri<'static, PB0>,
    pub channel_pulmonary_preload_pressure: Peri<'static, PB1>,
    pub channel_pulmonary_afterload_pressure: Peri<'static, PB11>,
}

impl Hal {
    pub fn new(p: Peripherals) -> Self {
        let mut adc1 = Adc::new(p.ADC1);

        // Medium sample time
        // TODO: tweak to sensor signal impedance
        adc1.set_sample_time(SampleTime::CYCLES47_5);

        let mut temp_channel = adc1.enable_temperature();
        let measured = adc1.blocking_read(&mut temp_channel);
        info!("measured temperature: {}", measured);

        let adc2 = Adc::new(p.ADC2);
        // let dac1 = Dac::new(p.DAC1, DacChannel::);
        // let dac2 = Dac::new(p.DAC2, DacChannel::C2);
        let led = Output::new(p.PA5, Level::Low, Speed::Low);

        let adc_channels = AdcChannels {
            channel_regulator_actual_pressure: p.PA0,
            channel_systemic_flow: p.PA1,
            channel_pulmonary_flow: p.PA2,
            channel_systemic_preload_pressure: p.PA3,
            channel_systemic_afterload_pressure: p.PB0,
            channel_pulmonary_preload_pressure: p.PB1,
            channel_pulmonary_afterload_pressure: p.PB11,
        };

        let button = Input::new(p.PC13, Pull::Down);

        Self {
            adc1,
            adc2,
            // dac1,
            // dac2,
            led,
            adc_channels,
            button,
        }
    }
}
