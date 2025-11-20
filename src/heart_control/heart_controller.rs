use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch};
use embassy_time::{Duration, Instant, Timer};
use love_letter::{AppState, Setpoint};
use uom::si::{f32::Pressure, pressure::bar};

use crate::{
    comms::task::CONNECTION_STATE,
    dac_task::DAC_REGULATOR_PRESSURE_WATCH,
    heart_control::phase::CardiacPhase,
    valve_task::{LEFT_VALVE_WATCH, RIGHT_VALVE_WATCH, ValveState},
};

/// Pneumatic heart controller routine
#[embassy_executor::task]
pub async fn heart_control_loop(mut setpoint_rx: watch::Receiver<'static, Cs, Setpoint, 3>) {
    info!("starting HEART CONTROL task");

    // Time spent in current cardiac phase
    let mut time_in_phase = Duration::from_micros(0);
    // Current cardiac phase of the heart
    let mut current_phase = CardiacPhase::Systole;

    let connection_state_rx = CONNECTION_STATE
        .receiver()
        .expect("Update CONNECTION_STATE N");

    let regulator_pressure_tx = DAC_REGULATOR_PRESSURE_WATCH.sender();
    let valve_left_tx = LEFT_VALVE_WATCH.sender();
    let valve_right_tx = RIGHT_VALVE_WATCH.sender();

    info!("HEART CONTROL: Moving mockloop into safe state");
    to_safe_heart_state(&regulator_pressure_tx, &valve_left_tx, &valve_right_tx);

    info!("HEART CONTROL: Waiting for initial setpoint");
    // Current setpoint
    let mut setpoint = setpoint_rx.changed().await;

    // Previous time state used to calculate time spent in current cardiac phase
    let mut prev_time = Instant::now();

    info!("HEART CONTROL: starting loop");
    loop {
        // Only control the heart if the heart controller is enabled
        if let Some(ref heart_setpoint) = setpoint.heart_controller_setpoint {
            // Update time spent in current phase
            time_in_phase += Instant::now() - prev_time;
            debug!("HEART CONTROL: time spent in phase: {}", time_in_phase);

            // Calculate total time we need to spend in current phase
            let mut total_phase_time = current_phase
                .get_total_phase_time(heart_setpoint.heart_rate, heart_setpoint.systole_ratio);

            debug!(
                "HEART CONTROL: Total to spend in phase: {}",
                total_phase_time
            );

            // Are we ready to switch to a new cardiac phase?
            if time_in_phase >= total_phase_time {
                // We are, make the switch
                current_phase = current_phase.switch();
                debug!(
                    "HEART CONTROL: switching cardiac phase to {:?}",
                    current_phase
                );

                // We entered a new phase: redo the total phase time calculation
                total_phase_time = current_phase
                    .get_total_phase_time(heart_setpoint.heart_rate, heart_setpoint.systole_ratio);

                // Reset time spend in current phase
                time_in_phase = Duration::from_micros(0);
            }

            // Control actuators to effect current cardiac phase
            actuate_cardiac_phase(
                &current_phase,
                heart_setpoint.pressure,
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
                    debug!(
                        "HEART CONTROL: Received a new setpoint from host: {:?}",
                        new_setpoint
                    );
                    // update current setpoint and continue
                    setpoint = new_setpoint;
                }
            }
        } else {
            // Heart Controller is disabled: Set the valves and pressure regulator into safe state
            debug!("HEART CONTROL: DISABLED -> Moving to safe state and ready for more action");

            to_safe_heart_state(&regulator_pressure_tx, &valve_left_tx, &valve_right_tx);

            // Await a new setpoint
            setpoint = setpoint_rx.changed().await;
        }
    }
}

/// Heart Controller is enabled: Actuate the valve and pressure regulator
async fn actuate_cardiac_phase(
    current_phase: &CardiacPhase,
    pressure: Pressure,
    pressure_tx: &watch::Sender<'static, Cs, Pressure, 1>,
    valve_left_tx: &watch::Sender<'static, Cs, ValveState, 1>,
    valve_right_tx: &watch::Sender<'static, Cs, ValveState, 1>,
) {
    // Find out the required solenoid valve states
    let left_valve_setpoint = get_valve_state_for_cardiac_phase(current_phase);
    let right_valve_setpoint = get_valve_state_for_cardiac_phase(current_phase);

    debug!(
        "HEART CONTROL: Enabled - actuating valves ({:?}, {:?}) and pressure regulator ({:?}bar)",
        left_valve_setpoint,
        right_valve_setpoint,
        pressure.get::<bar>()
    );

    // Actuate the pressure regulator
    control_pressure_regulator(pressure, pressure_tx);

    // Actuate the ventricle valves according to the current cardiac phase
    control_ventricle_valves(
        left_valve_setpoint,
        right_valve_setpoint,
        valve_left_tx,
        valve_right_tx,
    )
}

/// Set pressure regulator to the latest setpoint received for it
fn control_pressure_regulator(pressure: Pressure, tx: &watch::Sender<'static, Cs, Pressure, 1>) {
    trace!(
        "Controlling regulator pressure to: {:?}bar",
        pressure.get::<bar>()
    );

    tx.send(pressure);
}

fn control_ventricle_valves(
    left_valve_setpoint: ValveState,
    right_valve_setpoint: ValveState,
    valve_left_tx: &watch::Sender<'static, Cs, ValveState, 1>,
    valve_right_tx: &watch::Sender<'static, Cs, ValveState, 1>,
) {
    trace!(
        "Controlling valves to: [{:?}, {:?}]",
        left_valve_setpoint, right_valve_setpoint
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
    /// 0 bar pressure seems like the safest state for the solenoid
    const REGULATOR_SAFE_PRESSURE_BAR: f32 = 0.0;
    /// Safest solenoid state. Alternative is Vacuum which seems less safe
    const SAFE_SOLENOID_STATE: ValveState = ValveState::Pressure;

    trace!("HEART CONTROL: to SAFE state",);

    control_pressure_regulator(
        Pressure::new::<bar>(REGULATOR_SAFE_PRESSURE_BAR),
        pressure_tx,
    );
    control_ventricle_valves(
        SAFE_SOLENOID_STATE,
        SAFE_SOLENOID_STATE,
        valve_left_tx,
        valve_right_tx,
    )
}

/// Given the current set of measurements and previous state, what is our current state?
fn calculate_appstate() -> AppState {
    AppState::StandBy
}

fn get_valve_state_for_cardiac_phase(phase: &CardiacPhase) -> ValveState {
    match phase {
        CardiacPhase::Systole => ValveState::Pressure,
        CardiacPhase::Diastole => ValveState::Vacuum,
    }
}
