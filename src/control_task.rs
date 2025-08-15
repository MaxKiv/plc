use defmt::*;
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex as Cs, channel::Receiver, watch::Sender,
};
use embassy_time::{Duration, Ticker};

use crate::{AppState, comms::messages::AdcFrame};

const CONTROL_TASK_PERIOD: Duration = Duration::from_millis(10);

#[embassy_executor::task]
pub async fn control_loop(
    frame_in: Receiver<'static, Cs, AdcFrame, 2>,
    appstate_sender: Sender<'static, Cs, AppState, 1>,
) {
    // Task timekeeper
    let mut ticker = Ticker::every(CONTROL_TASK_PERIOD);

    let mut cnt = 0;
    let mut state = AppState::StandBy;

    loop {
        cnt += 1;

        if cnt == 100 {
            state = state.next();
            info!("CONTROL: cycling app_state to {}", state);

            appstate_sender.send(state);
        }

        debug!("CONTROL: looping");
        ticker.next().await;
    }
}
