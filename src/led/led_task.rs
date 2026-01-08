use defmt::*;
use embassy_stm32::{peripherals::TIM17, time::Hertz, timer::simple_pwm::SimplePwm};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Receiver};
use embassy_time::Duration;

use crate::AppState;

/// Led Brightness [0-100] %
struct Brightness(usize);

/// Period at which this task is ticked
const LED_TASK_TICK_PERIOD: Duration = Duration::from_millis(100);
const STANDBY_FREQUENCY: Hertz = Hertz(1);
const FAULT_FREQUENCY: Hertz = Hertz(1);

const DEFAULT_DUTY_CYCLE: u8 = 50;
const FAULT_DUTY_CYCLE: u8 = 100;

#[embassy_executor::task]
pub async fn blink_led(
    mut led: SimplePwm<'static, TIM17>,
    mut appstate_receiver: Receiver<'static, Cs, AppState, 1>,
) {
    info!("starting LED task");

    loop {
        // Wait for a new application state
        let new_app_state = appstate_receiver.changed().await;

        debug!(
            "LED: New app state detected - switched to {:?}",
            new_app_state
        );

        match new_app_state {
            AppState::StandBy => {
                led.set_frequency(STANDBY_FREQUENCY);
                led.ch1().set_duty_cycle_percent(DEFAULT_DUTY_CYCLE);
            }
            AppState::Running(freq) => {
                led.set_frequency(Hertz(freq));
                led.ch1().set_duty_cycle_percent(DEFAULT_DUTY_CYCLE);
            }
            AppState::Fault => {
                led.set_frequency(FAULT_FREQUENCY);
                led.ch1().set_duty_cycle_percent(FAULT_DUTY_CYCLE);
            }
        }

        led.ch1().enable();
    }
}
