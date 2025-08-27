use defmt::*;
use embassy_stm32::{
    mode::Async,
    usart::{UartRx, UartTx},
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex as Cs,
    watch::{self},
};
use embassy_time::{Duration, Ticker, WithTimeout};
use love_letter::{SETPOINT_BYTES, deserialize_setpoint, serialize_report};

use crate::{Report, Setpoint};

/// Period at which this task is ticked
const TASK_PERIOD: Duration = Duration::from_millis(100);
/// Time we remain patient before deciding the host is gone and we need to take matters into our
/// own hands
const SETPOINT_RECEIVE_TIMEOUT: Duration = Duration::from_millis(2000);
/// Time we remain patient before deciding our application is not responding
const REPORT_RECEIVE_TIMEOUT: Duration = Duration::from_millis(2000);

#[embassy_executor::task]
/// Forward firmware state reports to the HHH host every
pub async fn forward_reports(
    mut uart_tx: UartTx<'static, Async>,
    mut report_receiver: watch::Receiver<'static, Cs, Report, 1>,
) {
    let mut ticker = Ticker::every(TASK_PERIOD);

    loop {
        // Wait to receive a new report
        if let Ok(ref report) = report_receiver
            .changed()
            .with_timeout(REPORT_RECEIVE_TIMEOUT)
            .await
        {
            // Serialise received report
            let mut buf = [0u8; love_letter::REPORT_BYTES];
            match serialize_report(report.clone(), &mut buf) {
                Ok(buf) => {
                    // Send serialized report to host
                    info!(
                        "COMMS - forward_reports: sending serialised report to host: {:?}",
                        buf
                    );
                    if let Err(err) = uart_tx.write(buf).await {
                        error!(
                            "COMMS - forward_reports: Error sending uart frame {:?}: {}",
                            buf, err
                        );
                    }
                }
                Err(err) => {
                    error!(
                        "COMMS - forward_reports: Error serializing report {:?}: {}",
                        report, err
                    );
                }
            }
        } else {
            // our firmware seems to be toast
            error!(
                "COMMS - forward_reports: timeout waiting for report to arrive from control task, nothing to do but continue...",
            );
        }
        ticker.next().await;
    }
}

#[embassy_executor::task]
/// Process any new setpoints received from the host as soon as they come in
pub async fn receive_setpoints(
    mut uart_rx: UartRx<'static, Async>,
    setpoint_sender: watch::Sender<'static, Cs, Setpoint, 1>,
) {
    loop {
        // Receive setpoint
        let mut buf = [0u8; SETPOINT_BYTES];
        if let Ok(uart_result) = uart_rx
            .read(&mut buf)
            .with_timeout(SETPOINT_RECEIVE_TIMEOUT)
            .await
        {
            match uart_result {
                Ok(_) => {
                    info!(
                        "COMMS - receive_setpoints: read {} bytes ({})",
                        buf.len(),
                        buf
                    );

                    match deserialize_setpoint(&mut buf) {
                        Ok(setpoint) => {
                            info!(
                                "COMMS - receive_setpoints: sending deserialised setpoint {:?}",
                                setpoint
                            );

                            setpoint_sender.send(setpoint);
                        }
                        Err(err) => {
                            error!(
                                "COMMS - receive_setpoints: error deserialising setpoint from host: {}, skipping...",
                                err
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "COMMS - receive_setpoints: error receiving setpoint from host: {}, skipping...",
                        e
                    );
                }
            }
        } else {
            // our host seems to be toast
            error!(
                "COMMS - receive_setpoints: timeout waiting for setpoint from RPI3 host, nothing to do but continue...",
            );
        }
    }
}
