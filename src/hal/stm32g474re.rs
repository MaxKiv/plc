use embassy_stm32::adc::{Adc, SampleTime};
// use embassy_stm32::dac::{Dac, DacChannel};
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::mode::Async;
use embassy_stm32::rtc::{Rtc, RtcConfig};
use embassy_stm32::usart::{self, Uart};
use embassy_stm32::{
    Peri, Peripherals, bind_interrupts,
    peripherals::{self, *},
};

bind_interrupts!(struct Irqs {
    USART2 => usart::InterruptHandler<peripherals::USART2>;
});

/// Concrete HAL for STM32G474RE
pub struct Hal {
    pub adc1: Adc<'static, ADC1>,
    pub adc2: Adc<'static, ADC2>,
    // pub dac1: Dac<'static, DAC1>,
    // pub dac2: Dac<'static, DAC2>,
    pub dma: Peri<'static, DMA1_CH1>,
    pub led: Output<'static>,
    pub adc_channels: AdcChannels,
    pub button: Input<'static>,
    pub uart: Uart<'static, Async>,
    pub rtc: Rtc,
}

/// Number of adc inputs, this could be a fancy macro but I decided against the complexity
pub const NUM_ADC_INPUTS: usize = 7;

pub struct AdcChannels {
    pub regulator_actual_pressure: Peri<'static, PA0>,
    pub systemic_flow: Peri<'static, PA1>,
    pub pulmonary_flow: Peri<'static, PA2>,
    pub systemic_preload_pressure: Peri<'static, PA3>,
    pub systemic_afterload_pressure: Peri<'static, PB0>,
    pub pulmonary_preload_pressure: Peri<'static, PB1>,
    pub pulmonary_afterload_pressure: Peri<'static, PB11>,
}

impl Hal {
    pub fn new(p: Peripherals) -> Self {
        let mut adc1 = Adc::new(p.ADC1);

        // Medium sample time
        // TODO: tweak to sensor signal impedance
        adc1.set_sample_time(SampleTime::CYCLES47_5);

        // let mut temp_channel = adc1.enable_temperature();
        // let measured = adc1.blocking_read(&mut temp_channel);
        // info!("measured temperature: {}", measured);

        let adc2 = Adc::new(p.ADC2);
        // let dac1 = Dac::new(p.DAC1, DacChannel::);
        // let dac2 = Dac::new(p.DAC2, DacChannel::C2);
        let led = Output::new(p.PA5, Level::Low, Speed::Low);

        let adc_channels = AdcChannels {
            regulator_actual_pressure: p.PA0,
            systemic_flow: p.PA1,
            pulmonary_flow: p.PA2,
            systemic_preload_pressure: p.PA3,
            systemic_afterload_pressure: p.PB0,
            pulmonary_preload_pressure: p.PB1,
            pulmonary_afterload_pressure: p.PB11,
        };

        let dma = p.DMA1_CH1;

        let button = Input::new(p.PC13, Pull::Down);

        let uart = Uart::new(
            p.USART2,
            p.PB4,
            p.PB3,
            Irqs,
            p.DMA2_CH1,
            p.DMA2_CH2,
            usart::Config::default(),
        )
        .unwrap();

        // Default initialize the RTC
        let rtc = Rtc::new(p.RTC, RtcConfig::default());

        Self {
            adc1,
            adc2,
            // dac1,
            // dac2,
            dma,
            led,
            adc_channels,
            button,
            uart,
            rtc,
        }
    }
}
