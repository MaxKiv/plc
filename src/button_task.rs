use defmt::*;
use embassy_stm32::gpio::Input;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Sender};
use embassy_time::{Duration, Ticker, Timer};

use crate::AppState;

const TASK_PERIOD: Duration = Duration::from_millis(100);
const DEBOUNCE_DURATION: Duration = Duration::from_millis(70);

#[embassy_executor::task]
pub async fn manage_button(
    appstate_sender: Sender<'static, Cs, AppState, 1>,
    button: Input<'static>,
) {
    // Task timekeeper
    let mut ticker = Ticker::every(TASK_PERIOD);

    let mut state = AppState::default();

    loop {
        if button.is_high() {
            state = state.next();
            info!("CONTROL: cycling app_state to {}", state);
            appstate_sender.send(state);

            // simple debounce
            Timer::after(DEBOUNCE_DURATION).await;
        }

        ticker.next().await;
    }
}
