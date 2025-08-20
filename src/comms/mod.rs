use crate::{Report, Setpoint};

pub mod messages;
mod uart;

pub trait Transport {
    fn send_report(report: Report);
    fn receive_setpoint(setpoint: Setpoint);
}
