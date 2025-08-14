#![no_std]
#![no_main]

pub mod hal;
pub mod led_task;

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use panic_probe as _;

use crate::hal::Hal;

static HAL: Forever<Hal> = Forever::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let p = embassy_stm32::init(Default::default());

    info!("Default configuration applied");
    let mut hal = Hal::new(p);

    info!("Board specific HAL constructed");
    info!("Spawning tasks...");
    spawner.spawn(led_task::blink_led(hal.led)).unwrap();
}
