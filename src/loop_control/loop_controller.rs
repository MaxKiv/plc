use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch};
use love_letter::Setpoint;

/// Mockloop control loop
/// This control mockloop parameters like systemic/pulmonary flow resistance and compliance
#[embassy_executor::task]
pub async fn mockloop_control_loop(mut setpoint_rx: watch::Receiver<'static, Cs, Setpoint, 3>) {}
