use defmt::*;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, channel, watch};
use embassy_time::{Duration, Ticker};
use love_letter::{AppState, Report, Setpoint};

use crate::adc_task::AdcFrame;

/// Period at which this task is ticked
const CONTROL_TASK_PERIOD: Duration = Duration::from_millis(10);

/// Emergency stop routine
/// Pneumatic heart controller routine
/// Mockloop controller routine
/// Parses ADC frames into coherent [`Report`]s
#[embassy_executor::task]
pub async fn control_loop(
    frame_in: channel::Receiver<'static, Cs, AdcFrame, 2>,
    appstate_out: watch::Sender<'static, Cs, AppState, 1>,
    report_out: watch::Sender<'static, Cs, Report, 1>,
    mut setpoint_in: watch::Receiver<'static, Cs, Setpoint, 1>,
) {
    info!("starting CONTROL task");

    let mut ticker = Ticker::every(CONTROL_TASK_PERIOD);

    loop {
        if let Ok(frame) = frame_in.try_receive() {
            info!("CONTROL: received new adc frame: {:?}", frame);

            let setpoint = match setpoint_in.try_get() {
                Some(setpoint) => {
                    info!("CONTROL: received setpoint {:?}", setpoint);
                    setpoint
                }
                None => {
                    warn!("CONTROL: unable to collect latest setpoint, using default");
                    Setpoint::default()
                }
            };

            // Collect mockloop state and latest measurements into a report
            let report = Report {
                setpoint,
                app_state: calculate_appstate(),
                measurements: frame.into_measurement(),
            };
            info!("CONTROL: collected report: {:?}", report);

            // Send report to the host
            report_out.send(report);
        }

        debug!("CONTROL: looping");
        ticker.next().await;
    }
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
