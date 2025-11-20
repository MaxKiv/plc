use defmt::*;
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Receiver};
use embassy_time::{Duration, Ticker};

use crate::AppState;

/// Period at which this task is ticked
const LED_TASK_TICK_PERIOD: Duration = Duration::from_millis(100);

fn get_led_blink_period(app_state: AppState) -> Duration {
    match app_state {
        AppState::StandBy => Duration::from_millis(500),
        AppState::Running => Duration::from_millis(250),
        AppState::Fault => Duration::MAX,
    }
}

#[embassy_executor::task]
pub async fn blink_led(
    mut led: Output<'static>,
    mut appstate_receiver: Receiver<'static, Cs, AppState, 1>,
) {
    info!("starting LED task");

    // Task timekeeper
    let mut ticker = Ticker::every(LED_TASK_TICK_PERIOD);

    let mut current_app_state = AppState::default();
    let mut remaining_task_period = Some(get_led_blink_period(current_app_state));

    debug!("starting LED loop");

    loop {
        // Check if there is a new application state
        if let Some(new_app_state) = appstate_receiver.try_changed() {
            remaining_task_period = None;
            current_app_state = new_app_state;
            debug!(
                "LED: New app state detected - switched to {:?} - reset LED cycle",
                new_app_state
            );
        }

        if let Some(remaining) = remaining_task_period {
            remaining_task_period = remaining.checked_sub(LED_TASK_TICK_PERIOD);
        }

        trace!("LED: remaining led period {}", remaining_task_period);

        if remaining_task_period.is_none() {
            trace!("LED: current led cycle finished: toggling LED");
            led.toggle();
            remaining_task_period = Some(get_led_blink_period(current_app_state));
            trace!("LED: new remaining led period: {}", remaining_task_period);
        }

        trace!("LED: looping");
        ticker.next().await;
    }
}
