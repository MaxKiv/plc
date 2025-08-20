use embassy_stm32::mode::Async;
use love_letter::{Report, Setpoint};

use crate::comms::Transport;

pub struct Uart {
    uart: embassy_stm32::usart::Uart<'static, Async>,
}

impl Transport for Uart {
    fn send_report(report: Report) {
        todo!()
    }

    fn receive_setpoint(setpoint: Setpoint) {
        todo!()
    }
}
