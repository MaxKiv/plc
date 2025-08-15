#![no_std]
#![no_main]

mod adc_task;
mod comms;
mod comms_task;
mod control_task;
pub mod hal;
pub mod led_task;

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::Config;
use embassy_sync::channel::Channel;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Watch};
use panic_probe as _;
use serde::Serialize;

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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL85,
            divp: None,
            divq: None,
            // Main system clock at 170 MHz
            divr: Some(PllRDiv::DIV2),
        });
        config.rcc.mux.adc12sel = mux::Adcsel::SYS;
        config.rcc.sys = Sysclk::PLL1_R;
    }
    let p = embassy_stm32::init(config);

    info!("Default configuration applied");
    let hal = Hal::new(p);
    info!("Board specific HAL constructed");

    info!("Starting Application in AppState::Standby");
    APPSTATE_WATCH.sender().send(AppState::StandBy);

    info!("Spawning tasks...");
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
            hal.adc_channels,
            ADC_CHAN.sender(),
        ))
        .unwrap();
    spawner
        .spawn(control_task::control_loop(
            ADC_CHAN.receiver(),
            APPSTATE_WATCH.sender(),
        ))
        .unwrap();
}
