#![no_std]
#![no_main]

mod adc_task;
mod button_task;
mod comms;
mod comms_task;
mod control_task;
pub mod hal;
pub mod led_task;

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hsi48Config, LsConfig, RtcClockSource, Sysclk, mux,
};
use embassy_sync::channel::Channel;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Watch};
use panic_probe as _;
use serde::Serialize;

use crate::comms::messages::{Report, Setpoint};
use crate::{comms::messages::AdcFrame, hal::Hal};

#[derive(PartialEq, Clone, Copy, Serialize, Format, Default)]
enum AppState {
    #[default]
    StandBy,
    Running,
    Fault,
}

impl AppState {
    fn next(self) -> Self {
        match self {
            AppState::StandBy => AppState::Running,
            AppState::Running => AppState::Fault,
            AppState::Fault => AppState::StandBy,
        }
    }
}

static ADC_CHAN: Channel<Cs, AdcFrame, 2> = Channel::new();
static APPSTATE_WATCH: Watch<Cs, AppState, 1> = Watch::new();
static REPORT_WATCH: Watch<Cs, Report, 1> = Watch::new();
static SETPOINT_WATCH: Watch<Cs, Setpoint, 1> = Watch::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");

    let mut config = Config::default();
    configure_rcc(&mut config);

    let p = embassy_stm32::init(config);
    info!("Default configuration applied");

    let hal = Hal::new(p);
    info!("Board specific HAL constructed");

    info!("Starting Application in AppState::Standby");
    APPSTATE_WATCH.sender().send(AppState::StandBy);

    info!("Spawning tasks...");
    spawner
        .spawn(button_task::manage_button(
            APPSTATE_WATCH.sender(),
            hal.button,
        ))
        .unwrap();
    spawner
        .spawn(led_task::blink_led(
            hal.led,
            APPSTATE_WATCH
                .receiver()
                .expect("Creating a watch receiver should work"),
        ))
        .unwrap();
    spawner
        .spawn(adc_task::read_adc(
            hal.adc1,
            hal.dma,
            hal.adc_channels,
            ADC_CHAN.sender(),
        ))
        .unwrap();
    spawner
        .spawn(control_task::control_loop(
            ADC_CHAN.receiver(),
            APPSTATE_WATCH.sender(),
            REPORT_WATCH.sender(),
        ))
        .unwrap();

    let (uart_tx, uart_rx) = hal.uart.split();
    spawner
        .spawn(comms_task::forward_reports(
            uart_tx,
            REPORT_WATCH.receiver().unwrap(),
        ))
        .unwrap();
    spawner
        .spawn(comms_task::receive_setpoints(
            uart_rx,
            SETPOINT_WATCH.sender(),
        ))
        .unwrap();
}

// Configure reset and clock control
fn configure_rcc(config: &mut Config) {
    config.rcc.pll = None;
    config.rcc.hsi = true;
    config.rcc.hse = None;
    config.rcc.sys = Sysclk::HSI;
    config.rcc.hsi48 = Some(Hsi48Config {
        sync_from_usb: false,
    });
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV1;
    config.rcc.apb2_pre = APBPrescaler::DIV1;
    config.rcc.low_power_run = false;
    config.rcc.ls = LsConfig {
        rtc: RtcClockSource::LSI,
        lsi: true,
        lse: None,
    };
    config.rcc.boost = false;
    config.rcc.mux.rtcsel = mux::Rtcsel::LSI;
    config.rcc.mux.adc12sel = mux::Adcsel::SYS;
    config.rcc.mux.adc345sel = mux::Adcsel::SYS;
    config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
}
