use embassy_stm32::adc::Adc;
use embassy_stm32::dac::Dac;
use embassy_stm32::gpio::Output;
use embassy_stm32::peripherals::*;
use embedded_hal::adc::OneShot;
use embedded_hal::digital::v2::OutputPin;

/// Concrete HAL for STM32G474RE
pub struct Hal {
    pub adc1: Adc<'static, ADC1>,
    pub adc2: Adc<'static, ADC2>,
    pub dac1: Dac<'static, DAC1>,
    pub dac2: Dac<'static, DAC2>,
    pub led1: Output<'static, PA5>,
    pub led2: Output<'static, PA6>,
}

impl Hal {
    pub fn new(p: embassy_stm32::Peripherals) -> Self {
        let adc1 = Adc::new(p.ADC1, Default::default());
        let adc2 = Adc::new(p.ADC2, Default::default());
        let dac1 = Dac::new(p.DAC1, embassy_stm32::dac::DacChannel::C1);
        let dac2 = Dac::new(p.DAC2, embassy_stm32::dac::DacChannel::C2);
        let led1 = Output::new(p.PA5, embassy_stm32::gpio::Level::Low);
        let led2 = Output::new(p.PA6, embassy_stm32::gpio::Level::Low);
        Self {
            adc1,
            adc2,
            dac1,
            dac2,
            led1,
            led2,
        }
    }
}
