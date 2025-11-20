use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::dac::{Ch1, Dac, DacChannel};
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::mode::Async;
use embassy_stm32::rtc::{Rtc, RtcConfig};
use embassy_stm32::usart::{self, BufferedUart};
use embassy_stm32::{
    Peri, Peripherals, bind_interrupts,
    peripherals::{self, *},
};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    USART2 => usart::BufferedInterruptHandler<peripherals::USART2>;
});

static RX_BUF: StaticCell<[u8; 2048]> = StaticCell::new();
static TX_BUF: StaticCell<[u8; 2048]> = StaticCell::new();

/// Concrete HAL for STM32G474RE
pub struct Hal {
    pub adc1: Adc<'static, ADC1>,
    pub adc2: Adc<'static, ADC2>,
    pub pressure_regulator_dac: DacChannel<'static, DAC1, Ch1, Async>,
    pub left_valve: Output<'static>,
    pub right_valve: Output<'static>,
    pub dma: Peri<'static, DMA1_CH1>,
    pub led: Output<'static>,
    pub adc_channels: AdcChannels,
    pub button: Input<'static>,
    pub uart: BufferedUart<'static>,
    pub rtc: Rtc,
}

/// Number of adc inputs, this could be a fancy macro but I decided against the complexity
pub const NUM_ADC_INPUTS: usize = 7;

pub struct AdcChannels {
    pub regulator_actual_pressure: Peri<'static, PA0>,
    pub systemic_flow: Peri<'static, PA1>,
    pub pulmonary_flow: Peri<'static, PA2>,
    pub systemic_preload_pressure: Peri<'static, PC0>,
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
        let led = Output::new(p.PA5, Level::Low, Speed::Low);

        let adc_channels = AdcChannels {
            regulator_actual_pressure: p.PA0,
            systemic_flow: p.PA1,
            pulmonary_flow: p.PA2,
            systemic_preload_pressure: p.PC0,
            systemic_afterload_pressure: p.PB0,
            pulmonary_preload_pressure: p.PB1,
            pulmonary_afterload_pressure: p.PB11,
        };

        let dma = p.DMA1_CH1;

        let button = Input::new(p.PC13, Pull::Down);

        // Construct the BufferedUart, a structure allows us to process received uart bytes from a
        // ring buffer that is continously filled by DMA, and send uart bytes using a software FIFO
        let mut uart_cfg = usart::Config::default();
        // uart_cfg.baudrate = 921600;
        uart_cfg.baudrate = love_letter::BAUDRATE;
        let rx = p.PB4;
        let tx = p.PB3;
        let tx_buffer = &mut TX_BUF.init([0u8; 2048])[..];
        let rx_buffer = &mut RX_BUF.init([0u8; 2048])[..];
        let uart =
            BufferedUart::new(p.USART2, rx, tx, tx_buffer, rx_buffer, Irqs, uart_cfg).unwrap();

        // Default initialize the RTC
        let rtc = Rtc::new(p.RTC, RtcConfig::default());

        let (pressure_regulator_dac, systemic_compliance_dac) =
            Dac::new(p.DAC1, p.DMA1_CH3, p.DMA1_CH4, p.PA4, p.pa5).split();
        let pulmonary_compliance_dac = DacChannel::new(p.DAC2, p.DMA1_CH5, p.PA6);

        let left_valve = Output::new(p.PC2, Level::Low, Speed::Low);
        let right_valve = Output::new(p.PC3, Level::Low, Speed::Low);

        Self {
            adc1,
            adc2,
            pressure_regulator_dac,
            dma,
            led,
            adc_channels,
            button,
            uart,
            rtc,
            left_valve,
            right_valve,
        }
    }
}
