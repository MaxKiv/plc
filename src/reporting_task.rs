use defmt::*;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, channel, watch};
use embassy_time::{Duration, Ticker};
use love_letter::{AppState, Report, Setpoint};

use crate::adc_task::AdcFrame;

/// Minimum period between 2 reports
const REPORT_PERIOD: Duration = Duration::from_millis(100);

/// Parses latest ADC frames, Setpoints and AppState into coherent [`Report`]s
#[embassy_executor::task]
pub async fn collect_and_publish_reports(
    frame_in: channel::Receiver<'static, Cs, AdcFrame, 2>,
    report_out: watch::Sender<'static, Cs, Report, 1>,
    mut setpoint_rx: watch::Receiver<'static, Cs, Setpoint, 3>,
) {
    info!("starting REPORT task");
    let mut ticker = Ticker::every(REPORT_PERIOD);

    info!("starting REPORT loop");
    loop {
        // Wait for latest ADC frame, this is the most important part of the report
        let frame = frame_in.receive().await;
        // Get the latest known setpoint, or a default one if none is received yet
        // This might seem problematic, but during real operation any interesting adc
        // measurement has been accompanied by at least one previous setpoint
        let setpoint = setpoint_rx.try_get().unwrap_or_default();

        // Collect mockloop state and latest measurements into a report
        let report = Report {
            setpoint,
            app_state: calculate_appstate(),
            measurements: frame.into_measurement(),
        };

        info!("REPORT: collected report: {:?}", report);

        // Send report to the host
        report_out.send(report);

        trace!("REPORT: looping");
        // Crude attempt to slow down generated reports, this could be removed in the future
        ticker.next().await;
    }
}

/// Given the current set of measurements and previous state, what is our current state?
fn calculate_appstate() -> AppState {
    AppState::StandBy
}
