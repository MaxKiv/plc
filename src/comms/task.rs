use defmt::*;
use embassy_stm32::usart::{BufferedUartRx, BufferedUartTx};
use embassy_sync::watch::{self, Watch};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, pipe};
use embassy_time::{Duration, WithTimeout};
use embedded_io_async::Read;
use embedded_io_async::Write;

use crate::comms::connection_state::ConnectionState;

/// Period at which this task is ticked
const TASK_PERIOD: Duration = Duration::from_millis(10);
/// Time we remain patient before deciding the host is gone and we need to take matters into our
/// own hands
const SETPOINT_RECEIVE_TIMEOUT: Duration = Duration::from_millis(2000);
/// Time we remain patient before deciding our application is not responding
const REPORT_RECEIVE_TIMEOUT: Duration = Duration::from_millis(2000);

pub static CONNECTION_STATE: Watch<Cs, ConnectionState, 1> = Watch::new();

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
        info!("COMMS - forward_reports: writing {} bytes to UART", n);
        if let Err(err) = uart_tx.write_all(&buf[..n]).await {
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
    mut setpoint_pipe_tx: pipe::Writer<'static, Cs, { love_letter::SETPOINT_BYTES * 4 }>,
) {
    let mut buf = [0u8; 64];
    let tx = CONNECTION_STATE.sender();
    let mut connection_state = ConnectionState::Disconnected;

    loop {
        // Receive a full serialised setpoint message size worth of bytes
        match uart_rx
            .read(&mut buf)
            .with_timeout(SETPOINT_RECEIVE_TIMEOUT)
            .await
        {
            Ok(Ok(n)) => {
                trace!(
                    "COMMS - receive_setpoints: received setpoint {} bytes {:?}",
                    n,
                    buf[..n]
                );

                // Now we are talking!
                if connection_state != ConnectionState::Connected {
                    connection_state = ConnectionState::Connected;
                    tx.send(connection_state.clone());
                }

                // Yeet the setpoint bytes into a pipe for later deserialisation
                let _ = setpoint_pipe_tx.write_all(&buf[..n]).await;
            }
            Ok(Err(err)) => {
                error!(
                    "COMMS - receive_setpoints: {} error receiving setpoint from host, skipping...",
                    err
                );

                // Indicate issue
                if connection_state != ConnectionState::Stale {
                    connection_state = ConnectionState::Stale;
                    tx.send(connection_state.clone());
                }
            }
            Err(err) => {
                error!(
                    "COMMS - receive_setpoints: {} TIMEOUT receiving setpoint from host, I feel lonely :(",
                    err
                );

                // Track connection state
                connection_state = match connection_state {
                    ConnectionState::Connected => ConnectionState::Stale,
                    ConnectionState::Stale => ConnectionState::Disconnected,
                    ConnectionState::Disconnected => ConnectionState::Disconnected,
                };

                tx.send(connection_state.clone())
            }
        }
    }
}
