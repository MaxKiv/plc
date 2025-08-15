use crate::comms::{Transport, messages::*};

pub struct Uart {}

impl Transport for Uart {
    fn send_report(report: Report) {
        todo!()
    }

    fn receive_setpoint(setpoint: Setpoint) {
        todo!()
    }
}
