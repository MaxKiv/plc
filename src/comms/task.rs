use defmt::*;
use embassy_stm32::usart::{BufferedUartRx, BufferedUartTx};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, pipe};
use embedded_io_async::Read;
use embedded_io_async::Write;

#[embassy_executor::task]
/// Forward firmware state reports to the HHH host
pub async fn forward_reports(
    mut uart_tx: BufferedUartTx<'static>,
    report_pipe_rx: pipe::Reader<'static, Cs, { love_letter::REPORT_BYTES * 4 }>,
) {
    let mut buf = [0u8; 64];

    loop {
        // Get latest serialised report from the framing task
        let n = report_pipe_rx.read(&mut buf).await;
        debug!("COMMS - forward_reports: writing {} bytes to UART", n);
        if let Err(err) = uart_tx.write(&buf[..n]).await {
            error!(
                "COMMS - forward_reports: {} unable to write serialised report bytes {:?} to UART",
                err,
                buf[..n]
            );
        }
    }
}

#[embassy_executor::task]
/// Collects UART bytes into a pipe for later processing in framing_task
pub async fn receive_setpoints(
    mut uart_rx: BufferedUartRx<'static>,
    setpoint_pipe_tx: pipe::Writer<'static, Cs, { love_letter::SETPOINT_BYTES * 4 }>,
) {
    const BUF_SIZE: usize = 8;
    let mut buf = [0u8; BUF_SIZE];

    loop {
        if uart_rx.read_exact(&mut buf).await.is_ok() {
            info!(
                "COMMS - receive_setpoints: read {} bytes: {}",
                BUF_SIZE, buf
            );

            let mut written = 0;
            while written < BUF_SIZE {
                written += setpoint_pipe_tx.write(&buf[written..]).await;
            }

            debug!("COMMS - receive_setpoints: write {} bytes to pipe", written);
        }
    }
}
