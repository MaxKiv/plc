use defmt::*;
use embassy_stm32::{mode::Async, usart::Uart};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, channel::Receiver};
use postcard::to_slice;

use crate::comms::messages::Report;

#[embassy_executor::task]
pub async fn manage_communications(
    report_receiver: Receiver<'static, Cs, Report, 2>,
    mut uart: Uart<'static, Async>,
) {
    loop {
        // Receive report
        let report = report_receiver.receive().await;

        // Serialise received report
        let mut buf = [0u8; 32];
        let used = to_slice(&report, &mut buf).unwrap();

        // Send serialized report to host
        if let Err(e) = uart.write(used).await {
            error!("Error sending uart frame {:?}: {}", used, e);
        }

        // Receive setpoint
        let mut buf = [0u8; 32];
        let setpoint = uart.read_until_idle(&mut buf).await;
    }
}
