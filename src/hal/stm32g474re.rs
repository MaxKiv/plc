use embassy_stm32::adc::{Adc, AdcConfig};
use embassy_stm32::dac::{Ch1, Ch2, Dac, DacChannel};
use embassy_stm32::exti::{self};
use embassy_stm32::gpio::OutputType;
use embassy_stm32::interrupt;
use embassy_stm32::mode::Async;
// use embassy_stm32::rtc::{Rtc, RtcConfig};
use embassy_stm32::time::{Hertz, khz};
use embassy_stm32::timer::Channel;
use embassy_stm32::timer::complementary_pwm::ComplementaryPwm;
use embassy_stm32::timer::complementary_pwm::ComplementaryPwmPin;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::usart::{self, BufferedUart};
use embassy_stm32::{
    Peri, Peripherals, bind_interrupts,
    peripherals::{self, *},
};
use static_cell::StaticCell;

use crate::button_task::ButtonPeripherals;

bind_interrupts!(pub struct Irqs {
    USART2 => usart::BufferedInterruptHandler<peripherals::USART2>;
    EXTI15_10 => exti::InterruptHandler<interrupt::typelevel::EXTI15_10>;
});

static RX_BUF: StaticCell<[u8; 2048]> = StaticCell::new();
static TX_BUF: StaticCell<[u8; 2048]> = StaticCell::new();

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

/// Responsible for toggling the heart ventricle solenoid valves
/// Simple wrapper around [`ComplementaryPwm`] to easy switching of channels
pub struct ValvePwm {
    complementary_pwm: ComplementaryPwm<'static, TIM1>,
    channel: Channel,
}

impl ValvePwm {
    pub fn enable(&mut self) {
        self.complementary_pwm.enable(self.channel);
    }

    pub fn disable(&mut self) {
        self.complementary_pwm.disable(self.channel);
    }

    pub fn set_duty(&mut self, duty: u32) {
        self.complementary_pwm.set_duty(self.channel, duty);
    }

    pub fn set_frequency(&mut self, freq: Hertz) {
        self.complementary_pwm.set_frequency(freq);
    }

    pub fn set_frequency_low(&mut self, freq: f32) {
        self.complementary_pwm.set_frequency_low(freq);
    }

    pub fn get_max_duty(&self) -> u32 {
        self.complementary_pwm.get_max_duty()
    }
}

/// Concrete HAL for STM32G474RE
pub struct Hal {
    pub adc1: Adc<'static, ADC1>,
    pub adc2: Adc<'static, ADC2>,
    pub heart_pressure_dac: DacChannel<'static, DAC1, Ch1, Async>,
    pub systemic_compliance_dac: DacChannel<'static, DAC1, Ch2, Async>,
    pub pulmonary_compliance_dac: DacChannel<'static, DAC2, Ch1, Async>,
    pub valve_pwm: ValvePwm,
    pub dma: Peri<'static, DMA1_CH1>,
    pub led: SimplePwm<'static, TIM17>,
    pub adc_channels: AdcChannels,
    pub button: ButtonPeripherals<PC13>,
    pub uart: BufferedUart<'static>,
    pub irqs: Irqs,
}

impl Hal {
    pub fn new(p: Peripherals) -> Self {
        let mut adc1 = Adc::new(p.ADC1, AdcConfig::default());
        let adc2 = Adc::new(p.ADC2, AdcConfig::default());

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

        let led_pin = PwmPin::new(p.PB9, OutputType::PushPull);
        let led = SimplePwm::new(
            p.TIM17,
            Some(led_pin),
            None,
            None,
            None,
            Hertz(1),
            Default::default(),
        );

        let button = ButtonPeripherals {
            pin: p.PC13,
            ch: p.EXTI13,
        };

        // Construct the BufferedUart, a structure that allows us to process received uart bytes from a
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
        // let rtc = Rtc::new(p.RTC, RtcConfig::default());

        let (heart_pressure_dac, systemic_compliance_dac) =
            Dac::new(p.DAC1, p.DMA1_CH3, p.DMA1_CH4, p.PA4, p.PA5).split();
        let pulmonary_compliance_dac = DacChannel::new(p.DAC2, p.DMA1_CH5, p.PA6);

        let ch1 = PwmPin::new(p.PC2, OutputType::PushPull);
        let ch1n = ComplementaryPwmPin::new(p.PB15, OutputType::PushPull);
        let complementary_pwm = ComplementaryPwm::new(
            p.TIM1,
            None,
            None,
            None,
            None,
            Some(ch1),
            Some(ch1n),
            None,
            None,
            khz(10),
            Default::default(),
        );
        let valve_pwm = ValvePwm {
            complementary_pwm,
            channel: Channel::Ch3,
        };

        Self {
            adc1,
            adc2,
            heart_pressure_dac,
            systemic_compliance_dac,
            pulmonary_compliance_dac,
            dma,
            led,
            adc_channels,
            button,
            uart,
            valve_pwm,
            irqs: Irqs,
        }
    }
}
