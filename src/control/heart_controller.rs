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
    let valve_left_tx = LEFT_VALVE_WATCH.sender();
    let valve_right_tx = RIGHT_VALVE_WATCH.sender();

    info!("HEART CONTROL: Moving mockloop into safe state");
    to_safe_heart_state(regulator_pressure_tx, valve_left_tx, valve_right_tx);

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
        trace!("HEART CONTROL: time spent in phase: {time_in_phase}");

        // Calculate total time we need to spend in current phase
        let total_phase_time = current_phase.get_total_phase_time(
            heart_controller_setpoint.heart_rate,
            heart_controller_setpoint.systole_ratio,
        );

        trace!("HEART CONTROL: Total to spend in phase: {total_phase_time}");

        // Are we ready to switch to a new cardiac phase?
        if time_in_phase >= total_phase_time {
            // We are! Make the switch
            current_phase = current_phase.switch();
            trace!(
                "HEART CONTROL: switching cardiac phase to {:?}",
                current_phase
            );

            // We entered a new phase: redo the total phase time calculation
            total_phase_time = current_phase.get_total_phase_time(
                heart_controller_setpoint.heart_rate,
                heart_controller_setpoint.systole_ratio,
            );

            // Reset time spend in current phase
            time_in_phase = Duration::from_nanos(0);
        }

        // Control actuators to effect current cardiac phase
        actuate_cardiac_phase(
            current_phase,
            heart_controller_setpoint.pressure,
            &regulator_pressure_tx,
            &valve_left_tx,
            &valve_right_tx,
        )
        .await;

        // Timekeeping
        prev_time = Instant::now();

        // Now wait until either:
        // A: We are ready to switch cardiac phase again
        let wait_for_next_phase = Timer::after(total_phase_time);
        // B: We receive a new setpoint
        match select(wait_for_next_phase, setpoint_rx.changed()).await {
            // A: ready to switch cardiac phase
            Either::First(_) => {
                // time for next phase: continue
            }
            // B: Received a new setpoint; cancel wait and redo above calculations
            Either::Second(new_setpoint) => {
                trace!(
                    "HEART CONTROL: Received a new setpoint from host: {:?}",
                    new_setpoint
                );
                // update current setpoint and continue
                setpoint = new_setpoint;
            }
        }
    }
}

async fn actuate_cardiac_phase(
    current_phase: CardiacPhase,
    pressure: Pressure,
    pressure_tx: &watch::Sender<'static, Cs, Pressure, 1>,
    valve_left_tx: &watch::Sender<'static, Cs, ValveState, 1>,
    valve_right_tx: &watch::Sender<'static, Cs, ValveState, 1>,
) -> _ {
    if setpoint.enable {
        // Heart Controller is enabled: Actuate the valve and pressure regulator
        let hc = setpoint.heart_controller_setpoint;

        // Actuate the ventricle valves according to the current cardiac phase
        let (left_valve_setpoint, right_valve_setpoint) = match current_phase {
            CardiacPhase::Systole => (ValveState::Open, ValveState::Closed),
            CardiacPhase::Diastole => (ValveState::Closed, ValveState::Open),
        };

        trace!("HEART CONTROL: Active");

        control_pressure_regulator(hc.pressure, pressure_tx);

        control_ventricle_valves(valve_left_tx, valve_right_tx)
    } else {
        // Heart Controller is disabled: Set the valves and pressure regulator into safe state
        to_safe_heart_state(pressure_tx, valve_left_tx, valve_right_tx);
    }
}

/// Set pressure regulator to the latest setpoint received for it
fn control_pressure_regulator(pressure: Pressure, tx: &watch::Sender<'static, Cs, Pressure, 1>) {
    trace!("Controlling regulator pressure to: {:?}", pressure);

    tx.send(pressure);
}

fn control_ventricle_valves(
    valve_left_tx: &watch::Sender<'static, Cs, ValveState, 1>,
    valve_right_tx: &watch::Sender<'static, Cs, ValveState, 1>,
) {
    trace!(
        "Controlling valves to: [{:?}, {:?}]",
        valve_left_tx, valve_right_tx
    );

    valve_left_tx.send(left_valve_setpoint);
    valve_right_tx.send(right_valve_setpoint);
}

/// Sets the valves and pressure regulator into a safe state
fn to_safe_heart_state(
    pressure_tx: &watch::Sender<'static, Cs, Pressure, 1>,
    valve_left_tx: &watch::Sender<'static, Cs, ValveState, 1>,
    valve_right_tx: &watch::Sender<'static, Cs, ValveState, 1>,
) {
    const REGULATOR_SAFE_PRESSURE_BAR: f32 = 0.0;
    const VENTRICLE_SAFE_POSITION: bool = false;

    trace!("HEART CONTROL: to SAFE state",);

    control_pressure_regulator(
        Pressure::new::<bar>(REGULATOR_SAFE_PRESSURE_BAR),
        pressure_tx,
    );
    control_ventricle_valves(valve_left_tx, valve_right_tx)
}

/// Given the current set of measurements and previous state, what is our current state?
fn calculate_appstate() -> AppState {
    AppState::StandBy
}
