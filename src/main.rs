#![no_std]
#![no_main]

mod adc_task;
mod button_task;
mod comms_task;
mod control_task;
pub mod framing_task;
pub mod hal;
mod led_task;

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hsi48Config, LsConfig, PllMul, PllPreDiv, PllRDiv, PllSource,
    RtcClockSource, Sysclk, mux,
};
use embassy_stm32::{Config, rcc};
use embassy_sync::channel::Channel;
use embassy_sync::pipe::{self};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Watch};
use love_letter::{AppState, Report, Setpoint};
use panic_probe as _;
use static_cell::StaticCell;

use crate::adc_task::AdcFrame;
use crate::hal::Hal;

static ADC_CHAN: Channel<Cs, AdcFrame, 2> = Channel::new();
static APPSTATE_WATCH: Watch<Cs, AppState, 1> = Watch::new();
static REPORT_WATCH: Watch<Cs, Report, 1> = Watch::new();
static SETPOINT_WATCH: Watch<Cs, Setpoint, 1> = Watch::new();
static REPORT_PIPE: StaticCell<pipe::Pipe<Cs, { love_letter::REPORT_BYTES * 4 }>> =
    StaticCell::new();
static SETPOINT_PIPE: StaticCell<pipe::Pipe<Cs, { love_letter::SETPOINT_BYTES * 4 }>> =
    StaticCell::new();

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

    // Initialise serial communication pipes
    let report_pipe = REPORT_PIPE.init_with(pipe::Pipe::new);
    let setpoint_pipe = SETPOINT_PIPE.init_with(pipe::Pipe::new);
    let (report_pipe_rx, report_pipe_tx) = report_pipe.split();
    let (setpoint_pipe_rx, setpoint_pipe_tx) = setpoint_pipe.split();

    // Split UART into RX/TX halves
    let (uart_tx, uart_rx) = hal.uart.split();

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
        .spawn(comms_task::forward_reports(uart_tx, report_pipe_rx))
        .unwrap();
    spawner
        .spawn(comms_task::receive_setpoints(uart_rx, setpoint_pipe_tx))
        .unwrap();
    spawner
        .spawn(framing_task::serialise_reports(
            REPORT_WATCH.receiver().unwrap(),
            report_pipe_tx,
        ))
        .unwrap();
    spawner
        .spawn(framing_task::frame_and_serialise_setpoints(
            SETPOINT_WATCH.sender(),
            setpoint_pipe_rx,
        ))
        .unwrap();
    spawner
        .spawn(control_task::control_loop(
            ADC_CHAN.receiver(),
            APPSTATE_WATCH.sender(),
            REPORT_WATCH.sender(),
            SETPOINT_WATCH
                .receiver()
                .expect("max number of setpoint receivers created"),
        ))
        .unwrap();

    // Sleep the main task forever, Embassys docs are not clear on what happens when main returns
    // It seems the executor keeps running regardless
    // However lets not rely on undocumented behavior
    core::future::pending().await;
}

// Configure reset and clock control
fn configure_rcc(config: &mut Config) {
    // config.rcc.sys = Sysclk::HSI;
    config.rcc.sys = Sysclk::PLL1_R; // system clock comes from PLL1 R output
    config.rcc.pll = Some(embassy_stm32::rcc::Pll {
        source: PllSource::HSI,    // 16 MHz internal
        prediv: PllPreDiv::DIV1,   // 16 MHz in
        mul: PllMul::MUL21,        // 16 * 21 = 336 MHz VCO
        divp: None,                // not used
        divq: None,                // not used
        divr: Some(PllRDiv::DIV2), // 336 / 2 = 168 MHz SYSCLK
    });
    config.rcc.hsi = true;
    config.rcc.hse = None;
    config.rcc.hsi48 = Some(Hsi48Config {
        sync_from_usb: false,
    });
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV2;
    config.rcc.low_power_run = false;
    config.rcc.ls = LsConfig {
        rtc: RtcClockSource::LSI,
        lsi: true,
        lse: None,
    };
    config.rcc.boost = true;
    config.rcc.mux.rtcsel = mux::Rtcsel::LSI;
    config.rcc.mux.adc12sel = mux::Adcsel::SYS;
    config.rcc.mux.adc345sel = mux::Adcsel::SYS;
    config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
}
