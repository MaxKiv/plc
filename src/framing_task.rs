use defmt::*;

use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex as Cs,
    pipe::{self},
    watch,
};
use embassy_time::Timer;
use love_letter::{Report, Setpoint};

#[embassy_executor::task]
/// Deserialise the [`Report`]s collected from the control task into a UART byte stream to be
/// picked up by the comms task
pub async fn serialise_reports(
    mut report_receiver: watch::Receiver<'static, Cs, Report, 1>,
    report_pipe_tx: pipe::Writer<'static, Cs, { love_letter::REPORT_BYTES * 4 }>,
) {
    let mut buf = [0u8; love_letter::REPORT_BYTES * 2];
    loop {
        // Get latest report from the control task
        let report = report_receiver.changed().await;

        // Serialize it
        match love_letter::serialize_report(report.clone(), &mut buf) {
            Ok(mut serialised) => {
                // Push serialised report into pipe for consumption in comms task
                debug!(
                    "FRAMING - serialize_report: serialised report: {:?}",
                    serialised
                );
                // Write until full report is pushed into pipe
                while !serialised.is_empty() {
                    let n = report_pipe_tx.write(serialised).await;

                    if n == 0 {
                        // Pipe is full, yield until space is available
                        Timer::after(embassy_time::Duration::from_millis(1)).await;
                        continue;
                    }

                    serialised = &mut serialised[n..];
                }
            }
            Err(err) => {
                error!(
                    "FRAMING - serialise_reports: {} - Unable to serialise report {:?}, skipping...",
                    err, report
                );
            }
        }
    }
}

#[embassy_executor::task]
/// Frame the Pipe containing the UART byte stream from the comms task into [`Setpoint`]s and notify the control task
pub async fn frame_and_serialise_setpoints(
    setpoint_sender: watch::Sender<'static, Cs, Setpoint, 3>,
    setpoint_pipe_tx: pipe::Reader<'static, Cs, { love_letter::SETPOINT_BYTES * 4 }>,
) {
    let mut framing_buf = heapless::Vec::<u8, { love_letter::SETPOINT_BYTES * 4 }>::new();

    let mut buf = [0u8; 1];
    loop {
        // Reading a single byte at a time allows us to properly frame the incoming COBS encoded setpoints
        match setpoint_pipe_tx.read(&mut buf).await {
            0 => {
                debug!(
                    "FRAMING - frame_setpoints: Failed to read a byte from the pipe, retry next cycle"
                );
            }
            1 => {
                // Happy path - Read single byte
                let byte = buf[0];

                trace!(
                    "FRAMING - frame_setpoints: read byte from setpoint_pipe_tx: {}",
                    byte
                );

                if byte == 0 {
                    debug!(
                        "FRAMING - frame_setpoints: COBS delimiter detected, attempting to frame: {:?}",
                        framing_buf
                    );

                    // COBS delimiter byte: process frame
                    match love_letter::deserialize_setpoint(&mut framing_buf) {
                        Ok(setpoint) => {
                            info!(
                                "FRAMING - frame_setpoints: COBS delimeter detected & Deserialise succes: {:?}",
                                setpoint
                            );
                            // Happy path - Send deserialised setpoint to control task
                            setpoint_sender.send(setpoint);
                        }
                        Err(err) => {
                            error!(
                                "FRAMING - frame_setpoints: Unable to deserialise framing buffer into a setpoint. Err: {} - buffer: {:?}",
                                err, framing_buf
                            );
                        }
                    }
                    // Reset current frame
                    framing_buf.clear();
                } else {
                    trace!("FRAMING - frame_setpoints: data byte: {}", byte);
                    // Data byte: add to frame
                    if let Err(byte) = framing_buf.push(byte) {
                        error!(
                            "FRAMING - frame_setpoints: Unable to collect byte {} because framing buffer {:?} is full, should never happen but you are here anyway",
                            byte, framing_buf
                        );
                        // Clear frame, issue is hopefully resolved after next delimiter byte
                        framing_buf.clear();
                    }
                }
            }
            n => {
                error!(
                    "FRAMING - frame_setpoints: Read {} bytes, more bytes than fit in buffer? This should never happen",
                    n
                );
            }
        }
    }
}
