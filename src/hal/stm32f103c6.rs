use embassy_stm32::adc::Adc;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::*;
// use embedded_hal::adc::OneShot;
// use embedded_hal::digital::v2::OutputPin;

/// Concrete HAL for STM32F103C6
pub struct Hal {
    pub adc: Adc<'static, ADC1>,
    // NOTE: the STM32F103C6 has no DAC
    // pub dac: Dac<'static, DAC>,
    pub led: Output<'static>,
}

impl Hal {
    pub fn new(p: embassy_stm32::Peripherals) -> Self {
        let adc = Adc::new(p.ADC1);
        let led = Output::new(p.PB12, Level::Low, Speed::Low);
        Self { adc, led }
    }
}
