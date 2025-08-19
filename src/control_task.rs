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

    loop {
        if let Ok(frame) = frame_in.try_receive() {
            info!("CONTROL: received new adc frame: {:?}", frame);
        }

        debug!("CONTROL: looping");
        ticker.next().await;
    }
}
