use embassy_stm32::adc::Adc;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::mode::Async;
use embassy_stm32::usart::{self, Config, Uart};
use embassy_stm32::{
    Peri, bind_interrupts,
    peripherals::{self, *},
};

pub struct AdcChannels {
    pub channel_regulator_actual_pressure: Peri<'static, PA0>,
    pub channel_systemic_flow: Peri<'static, PA1>,
}

bind_interrupts!(struct Irqs {
    USART1 => usart::InterruptHandler<peripherals::USART1>;
});

/// Concrete HAL for STM32F103C6
pub struct Hal {
    pub adc1: Adc<'static, ADC1>,
    pub adc_channels: AdcChannels,
    // NOTE: the STM32F103C6 has no DAC
    // pub dac: Dac<'static, DAC>,
    pub led: Output<'static>,
    pub uart: Uart<'static, Async>,
}

impl Hal {
    pub fn new(p: embassy_stm32::Peripherals) -> Self {
        let adc1 = Adc::new(p.ADC1);

        // Medium sample time
        // TODO: tweak to sensor signal impedance
        adc1.set_sample_time(SampleTime::CYCLES28_5);

        let adc_channels = AdcChannels {
            channel_regulator_actual_pressure: p.PA0,
            channel_systemic_flow: p.PA1,
        };
        let led = Output::new(p.PB12, Level::Low, Speed::Low);
        let uart = Uart::new(
            p.USART1,
            p.PA10,
            p.PA9,
            Irqs,
            p.DMA1_CH4,
            p.DMA1_CH5,
            Config::default(),
        )
        .unwrap();

        Self {
            adc1,
            adc_channels,
            led,
            uart,
        }
    }
}
