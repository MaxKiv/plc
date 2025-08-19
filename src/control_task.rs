use defmt::*;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, channel, watch};
use embassy_time::{Duration, Ticker};

use crate::{
    AppState,
    comms::messages::{AdcFrame, Measurements, Report},
};

/// Period at which this task is ticked
const CONTROL_TASK_PERIOD: Duration = Duration::from_millis(10);

#[embassy_executor::task]
pub async fn control_loop(
    frame_in: channel::Receiver<'static, Cs, AdcFrame, 2>,
    appstate_out: watch::Sender<'static, Cs, AppState, 1>,
    report_out: watch::Sender<'static, Cs, Report, 1>,
) {
    info!("starting CONTROL task");

    let mut ticker = Ticker::every(CONTROL_TASK_PERIOD);

    loop {
        if let Ok(frame) = frame_in.try_receive() {
            info!("CONTROL: received new adc frame: {:?}", frame);

            // Collect mockloop state and latest measurements into a report
            let report = Report {
                app_state: calculate_appstate(),
                measurements: Measurements::from_frame(frame),
            };
            info!("CONTROL: collected report: {:?}", report);

            // Send report to the host
            report_out.send(report);
        }

        debug!("CONTROL: looping");
        ticker.next().await;
    }
}

/// Given the current set of measurements and previous state, what is our current state?
fn calculate_appstate() -> AppState {
    AppState::StandBy
}
