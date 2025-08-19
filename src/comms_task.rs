use defmt::*;
use embassy_stm32::{mode::Async, usart::Uart};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, channel::Receiver};
use embassy_time::{Duration, Ticker};
use postcard::to_slice;

use crate::comms::messages::Report;

const TASK_PERIOD: Duration = Duration::from_millis(100);

#[embassy_executor::task]
pub async fn manage_communications(
    report_receiver: Receiver<'static, Cs, Report, 2>,
    mut uart: Uart<'static, Async>,
) {
    info!("starting COMMS task");

    let mut ticker = Ticker::every(TASK_PERIOD);

    loop {
        // Receive report
        let report = report_receiver.receive().await;

        // Serialise received report
        let mut buf = [0u8; 32];
        let used = to_slice(&report, &mut buf).unwrap();

        info!("COMMS: sending serialised report to host: {:?}", used);

        // Send serialized report to host
        if let Err(e) = uart.write(used).await {
            error!("COMMS: Error sending uart frame {:?}: {}", used, e);
        }

        // Receive setpoint
        // let mut buf = [0u8; 32];
        // let setpoint = uart.read_until_idle(&mut buf).await;

        ticker.next().await;
    }
}
