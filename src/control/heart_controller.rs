use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_stm32::time::Hertz;
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex as Cs,
    channel,
    watch::{self, Sender},
};
use embassy_time::{Duration, Instant, Ticker, Timer};
use love_letter::{
    AppState, HeartControllerSetpoint, Measurements, MockloopSetpoint, Report,
    SYSTOLE_RATIO_DEFAULT, Setpoint,
};
use uom::si::{
    f32::{Frequency, Pressure},
    frequency::{cycle_per_minute, hertz},
    pressure::bar,
};

use crate::{
    comms::task::CONNECTION_STATE,
    control::{error::ControlError, phase::CardiacPhase},
    dac_task::DAC_REGULATOR_PRESSURE_WATCH,
};

/// Period at which this task is ticked
const CONTROL_TASK_PERIOD: Duration = Duration::from_millis(10);

/// Emergency stop routine
/// Pneumatic heart controller routine
/// Mockloop controller routine
/// Parses ADC frames into coherent [`Report`]s
#[embassy_executor::task]
pub async fn heart_control_loop(
    // appstate_out: watch::Sender<'static, Cs, AppState, 1>,
    // report_out: watch::Sender<'static, Cs, Report, 1>,
    mut setpoint_rx: watch::Receiver<'static, Cs, Setpoint, 1>,
) {
    info!("starting HEART CONTROL task");

    let mut ticker = Ticker::every(CONTROL_TASK_PERIOD);

    // Time spent in current cardiac phase
    let mut time_in_phase = Duration::from_nanos(0);
    let mut current_phase = CardiacPhase::Systole;

    let connection_state_rx = CONNECTION_STATE
        .receiver()
        .expect("Update CONNECTION_STATE N");
    let regulator_pressure_tx = DAC_REGULATOR_PRESSURE_WATCH.sender();

    info!("HEART CONTROL: Moving mockloop into safe state");
    to_safe_state();

    info!("HEART CONTROL: Waiting for initial setpoint");
    let setpoint @ Setpoint {
        enable,
        mockloop_setpoint,
        heart_controller_setpoint,
    } = setpoint_rx.changed().await;

    let mut prev_time = Instant::now();

    info!("starting HEART CONTROL loop");
    loop {
        // Update time spent in current phase
        time_in_phase += Instant::now() - prev_time;

        // Calculate total time we need to spend in current phase
        let total_phase_time = current_phase.get_total_phase_time(
            heart_controller_setpoint.heart_rate,
            heart_controller_setpoint.systole_ratio,
        );

        // Are we ready to switch to a new cardiac phase?
        if time_in_phase >= total_phase_time {
            // We are! Make the switch
            current_phase = current_phase.switch();

            // We entered a new phase: redo the total phase time calculation
            total_phase_time = current_phase.get_total_phase_time(
                heart_controller_setpoint.heart_rate,
                heart_controller_setpoint.systole_ratio,
            );

            // Reset time spend in current phase
            time_in_phase = Duration::from_nanos(0);
        }

        // Control actuators to effect current cardiac phase
        actuate_cardiac_phase(current_phase, setpoint, &regulator_pressure_tx).await;

        // Timekeeping
        prev_time = Instant::now();

        // Now wait until either:
        // A: We are ready to switch cardiac phase again
        let wait_for_next_phase = Timer::after(total_phase_time);
        // B: We receive a new setpoint
        let res = select(wait_for_next_phase, setpoint_rx.changed()).await;
        match res {
            // A: ready to switch cardiac phase
            Either::First(_) => { /* timed out: continue*/ }
            // B: Received a new setpoint; cancel wait and redo above calculations
            Either::Second(new_setpoint) => {
                /* update current setpoint and continue */
                setpoint = new_setpoint;
            }
        }
    }
}

async fn actuate_cardiac_phase(
    current_phase: CardiacPhase,
    setpoint: Setpoint,
    regulator_tx: &watch::Sender<'static, Cs, Pressure, 1>,
) -> _ {
    if setpoint.enable {
        let hc = setpoint.heart_controller_setpoint;
        control_pressure_regulator(hc.pressure, regulator_tx);
        control_ventricle_valves()
    } else {
    }
}

/// Set pressure regulator to the latest setpoint received for it
fn control_pressure_regulator(pressure: Pressure, tx: &watch::Sender<'static, Cs, Pressure, 1>) {
    debug!("Controlling regulator pressure to: {:?}", pressure);

    tx.send(pressure);
}

fn control_ventricle_valves(current_phase: CardiacPhase) {
    // Actuate the ventricle valves according to the current cardiac phase
    let (left_valve_setpoint, right_valve_setpoint) = match current_phase {
        CardiacPhase::Systole => (ValveState::Open, ValveState::Closed),
        CardiacPhase::Diastole => (ValveState::Closed, ValveState::Open),
    };
}

fn control_ventricles() {
    // Time bookkeeping
    let current_time = Instant::now();
    self.time_spent_in_current_phase += current_time - LAST_CYCLE_TIME;
    info!(
        "current heart rate setpoint {} cycles per minute",
        setpoint.heart_controller_setpoint.heart_rate.get::<hertz>()
    );

    debug!(
        "time spent in current cardiac phase: {:?}",
        self.time_spent_in_current_phase
    );

    // Check if its time to switch cardiac phase
    let current_cardiac_phase_duration = TimeDelta::from_std(Duration::from_secs_f64(
        1.0 / setpoint.heart_rate.get::<hertz>()
            * match self.current_cardiac_phase {
                CardiacPhase::Systole => setpoint.systole_ratio,
                CardiacPhase::Diastole => 1.0 - setpoint.systole_ratio,
            },
    ))
    .unwrap();

    debug!(
        "Current phase duration {:?}",
        current_cardiac_phase_duration
    );

    // Switch cardiac phase when necessary
    if self.time_spent_in_current_phase > current_cardiac_phase_duration {
        self.current_cardiac_phase = self.current_cardiac_phase.switch();
        self.time_spent_in_current_phase = TimeDelta::zero();
    }

    // Actuate the ventricle valves according to the current cardiac phase
    let (left_valve_setpoint, right_valve_setpoint) = match self.current_cardiac_phase {
        CardiacPhase::Systole => (ValveState::Open, ValveState::Closed),
        CardiacPhase::Diastole => (ValveState::Closed, ValveState::Open),
    };

    info!("Setting left valve: {:?}", left_valve_setpoint);
    info!("Setting right valve: {:?}", right_valve_setpoint);
    self.hw.set_valve(Valve::Left, left_valve_setpoint).unwrap();
    self.hw
        .set_valve(Valve::Right, right_valve_setpoint)
        .unwrap();
}

fn to_safe_state() {
    const REGULATOR_SAFE_PRESSURE_BAR: f32 = 0.0;
    const VENTRICLE_SAFE_POSITION: bool = false;
    false.control_pressure_regulator(Pressure::new::<bar>(REGULATOR_SAFE_PRESSURE_BAR));
    control_ventricles(&mut self, setpoint);
}

/// Given the current set of measurements and previous state, what is our current state?
fn calculate_appstate() -> AppState {
    AppState::StandBy
}

// TODO: reuse code below
//
//
// const DEFAULT_CONTROL_LOOP_PERIOD: Duration = Duration::from_millis(100);
//
// /// Setpoint for the mockloop controller
// #[derive(Clone, Deserialize, Serialize)]
// pub struct ControllerSetpoint {
//     /// Should the mockloop controller be enabled?
//     pub enable: bool,
//     /// Desired heart rate
//     pub heart_rate: Frequency,
//     /// Desired regulator pressure
//     pub pressure: Pressure,
//     /// Control loop Frequency
//     pub loop_frequency: Frequency,
//     /// Ratio of systole duration to total cardiac phase duration
//     /// NOTE: usually 3/7
//     pub systole_ratio: f64,
// }
//
// impl fmt::Debug for ControllerSetpoint {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_struct("Point")
//             .field("enable", &self.enable)
//             .field(
//                 "heart_rate (BPM)",
//                 &self.heart_rate.get::<cycle_per_minute>(),
//             )
//             .field("pressure (bar)", &self.pressure.get::<bar>())
//             .field("loop_frequency (Hz)", &self.loop_frequency.get::<hertz>())
//             .field("systole_ratio", &self.systole_ratio)
//             .finish()
//     }
// }
//
// impl Default for ControllerSetpoint {
//     fn default() -> Self {
//         Self {
//             enable: false,
//             heart_rate: Frequency::new::<hertz>(80.0 / 60.0),
//             pressure: Pressure::new::<bar>(0.0),
//             loop_frequency: Frequency::new::<hertz>(
//                 1.0 / DEFAULT_CONTROL_LOOP_PERIOD.as_secs_f64(),
//             ),
//             systole_ratio: 3.0 / 7.0,
//         }
//     }
// }
//
// /// Phases of the heart ventricles
// /// Systole = ventricle contraction, Diastole = ventricle relaxation
// #[derive(Debug)]
// enum CardiacPhases {
//     Systole,
//     Diastole,
// }
//
// impl CardiacPhases {
//     fn switch(&self) -> Self {
//         match self {
//             CardiacPhases::Systole => CardiacPhases::Diastole,
//             CardiacPhases::Diastole => CardiacPhases::Systole,
//         }
//     }
// }
//
// /// Mockloop controller state machine states
// #[derive(Debug)]
// enum ControllerState {
//     PreOp,
//     Op,
//     Err,
// }
//
// /// Controller for the Mockloop
// #[derive(Debug)]
// pub struct MockloopController<H: MockloopHardware> {
//     // Mockloop hardware interface
//     hw: H,
//     // Receives controller setpoints from other parts of the application
//     setpoint_receiver: Receiver<ControllerSetpoint>,
//     // Current mockloop controller state
//     state: ControllerState,
//     // Time at last cycle
//     last_cycle_time: DateTime<Utc>,
//     // Current cardiac phase
//     current_cardiac_phase: CardiacPhases,
//     // Time spent in current cardiac phase
//     time_spent_in_current_phase: TimeDelta,
// }
//
// impl<T> MockloopController<T>
// where
//     T: MockloopHardware,
// {
//     /// Initialize a new controller with the given hardware interface and setpoint receiver
//     pub fn new(hw: T, setpoint_receiver: Receiver<ControllerSetpoint>) -> Self {
//         info!("Initialize controller");
//         MockloopController {
//             state: ControllerState::PreOp,
//             last_cycle_time: Utc::now(),
//             current_cardiac_phase: CardiacPhases::Systole,
//             time_spent_in_current_phase: TimeDelta::zero(),
//             setpoint_receiver,
//             hw,
//         }
//     }
//
//     /// Run the MockloopController
//     #[instrument(skip(self))]
//     pub async fn run(mut self) {
//         // Obtain the initial controller setpoint
//         let initial_setpoint = self.setpoint_receiver.borrow().clone();
//
//         // Calculate the desired control loop interval
//         let period: f64 = 1.0 / initial_setpoint.loop_frequency.get::<hertz>();
//         let mut next_tick_time = Instant::now() + Duration::from_secs_f64(period);
//
//         // Run the control loop
//         loop {
//             // Fetch the latest available controller setpoint
//             let setpoint = self.setpoint_receiver.borrow().clone();
//             info!("current controller setpoint: {:?}", setpoint);
//             info!(
//                 "current heart rate setpoint {} cycles per minute",
//                 setpoint.heart_rate.get::<cycle_per_minute>()
//             );
//
//             // Use it to control the mockloop
//             if setpoint.enable {
//                 // Controller enabled -> tick the controller state machine
//                 self.tick(setpoint.clone()).await;
//             } else {
//                 // Set control loop to pre operation while controller is disabled
//                 self.state = ControllerState::PreOp;
//                 // Make sure mockloop is in safe position when disabled
//                 if let Err(err) = self.hw.to_safe_state() {
//                     error!("Unable to move mockloop into safe state: {}", err);
//                 }
//             }
//
//             // Time bookkeeping
//             self.last_cycle_time = Utc::now();
//             // trace!("control looping")
//
//             // Preempt until desired control loop interval has passed
//             tokio::time::sleep_until(next_tick_time).await;
//
//             let period: f64 = 1.0 / setpoint.loop_frequency.get::<hertz>();
//             next_tick_time += Duration::from_secs_f64(period);
//         }
//     }
//
//     /// Single tick of the controller state machine
//     pub async fn tick(&mut self, setpoint: ControllerSetpoint) {
//         match &self.state {
//             ControllerState::PreOp => self.preop(),
//             ControllerState::Op => self.op(setpoint),
//             ControllerState::Err => self.err(),
//         };
//     }
//
//     /// Pre operation logic, actuate mockloop into safe state, reset cardiac phase time tracking
//     /// and transition to Operational
//     fn preop(&mut self) {
//         debug!(state = "PREOP");
//
//         // Make sure the mockloop is in a safe state
//         self.hw.to_safe_state().unwrap();
//
//         // Reset the cardiac phase tracking
//         self.current_cardiac_phase = CardiacPhases::Systole;
//         self.time_spent_in_current_phase = TimeDelta::zero();
//
//         self.state = ControllerState::Op;
//     }
//
//     /// Error state logic, unrecoverable
//     fn err(&mut self) {
//         debug!(state = "ERR");
//
//         // Make sure the mockloop is in a safe state
//         self.hw.to_safe_state().unwrap();
//
//         self.state = ControllerState::Err;
//     }
//
//     /// Operational logic, control ventricles and pressure regulator
//     fn op(&mut self, setpoint: ControllerSetpoint) {
//         debug!(state = "OP");
//         self.control_pressure_regulator(setpoint.pressure);
//         self.control_ventricles(setpoint);
//     }
//
//     /// Set pressure regulator to the latest setpoint received for it
//     fn control_pressure_regulator(&mut self, pressure: Pressure) {
//         debug!(state = "OP", "Setting regulator pressure: {:?}", pressure);
//         if let Err(err) = self.hw.set_regulator_pressure(pressure) {
//             error!(
//                 "Unable to set controller regulator pressure to {:?} bar: {:?}",
//                 pressure.get::<bar>(),
//                 err
//             );
//             // An invalid regulator pressure setpoint was given, set to safe value
//             let _ = self
//                 .hw
//                 .set_regulator_pressure(Pressure::new::<bar>(REGULATOR_MIN_PRESSURE_BAR));
//             warn!(
//                 "Set controller regulator presesure to safe value: {:?} bar",
//                 REGULATOR_MIN_PRESSURE_BAR
//             );
//         }
//     }
//
//     /// Control the ventricle pneumatic valves in such a way the heartbeats at the desired heart rate
//     fn control_ventricles(&mut self, setpoint: ControllerSetpoint) {
//         // Time bookkeeping
//         let current_time = Utc::now();
//         self.time_spent_in_current_phase += current_time - self.last_cycle_time;
//         debug!(
//             "time spent in current cardiac phase: {:?}",
//             self.time_spent_in_current_phase
//         );
//
//         // Check if its time to switch cardiac phase
//         let current_cardiac_phase_duration = TimeDelta::from_std(Duration::from_secs_f64(
//             1.0 / setpoint.heart_rate.get::<hertz>()
//                 * match self.current_cardiac_phase {
//                     CardiacPhases::Systole => setpoint.systole_ratio,
//                     CardiacPhases::Diastole => 1.0 - setpoint.systole_ratio,
//                 },
//         ))
//         .unwrap();
//
//         debug!(
//             "Current phase duration {:?}",
//             current_cardiac_phase_duration
//         );
//
//         // Switch cardiac phase when necessary
//         if self.time_spent_in_current_phase > current_cardiac_phase_duration {
//             self.current_cardiac_phase = self.current_cardiac_phase.switch();
//             self.time_spent_in_current_phase = TimeDelta::zero();
//         }
//
//         // Actuate the ventricle valves according to the current cardiac phase
//         let (left_valve_setpoint, right_valve_setpoint) = match self.current_cardiac_phase {
//             CardiacPhases::Systole => (ValveState::Open, ValveState::Closed),
//             CardiacPhases::Diastole => (ValveState::Closed, ValveState::Open),
//         };
//
//         info!("Setting left valve: {:?}", left_valve_setpoint);
//         info!("Setting right valve: {:?}", right_valve_setpoint);
//         self.hw.set_valve(Valve::Left, left_valve_setpoint).unwrap();
//         self.hw
//             .set_valve(Valve::Right, right_valve_setpoint)
//             .unwrap();
//     }
// }
