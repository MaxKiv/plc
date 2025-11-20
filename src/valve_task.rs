use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::Output;
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex as Cs,
    watch::{self, Watch},
};

pub static LEFT_VALVE_WATCH: Watch<Cs, ValveState, 1> = Watch::new();
pub static RIGHT_VALVE_WATCH: Watch<Cs, ValveState, 1> = Watch::new();

#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum ValveState {
    Pressure,
    Vacuum,
}

pub struct Valve {
    pin: Output<'static>,
    state: ValveState,
    rx: watch::Receiver<'static, Cs, ValveState, 1>,
}

impl Valve {
    fn actuate(&mut self) {
        match self.state {
            ValveState::Pressure => self.pin.set_high(),
            ValveState::Vacuum => self.pin.set_low(),
        }
    }
}

#[embassy_executor::task]
pub async fn control_valves(left_valve_pin: Output<'static>, right_valve_pin: Output<'static>) {
    info!("starting VALVE task");

    let rx_left = LEFT_VALVE_WATCH
        .receiver()
        .expect("Increase left valve watch size");

    let mut left_valve = Valve {
        pin: left_valve_pin,
        state: ValveState::Vacuum,
        rx: rx_left,
    };

    let rx_right = RIGHT_VALVE_WATCH
        .receiver()
        .expect("Increase right valve watch size");
    let mut right_valve = Valve {
        pin: right_valve_pin,
        state: ValveState::Vacuum,
        rx: rx_right,
    };

    info!("starting VALVE loop");
    loop {
        // Wait for valve actuation request
        let left_valve_update = left_valve.rx.changed();
        let right_valve_update = right_valve.rx.changed();
        match select(left_valve_update, right_valve_update).await {
            // Left valve is supposed to be actuated, do so
            Either::First(new_state) => {
                // New setpoint for the left valve
                left_valve.state = new_state;

                left_valve.actuate()
            }
            // Right valve is supposed to be actuated, do so
            Either::Second(new_state) => {
                // New setpoint for the right valve
                right_valve.state = new_state;

                right_valve.actuate()
            }
        }
    }
}
