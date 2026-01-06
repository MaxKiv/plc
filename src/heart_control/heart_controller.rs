use defmt::*;
use embassy_stm32::time::Hertz;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch};
use love_letter::{AppState, HeartControllerSetpoint, Setpoint};
use uom::si::{f32::Pressure, frequency::hertz, pressure::bar};

use crate::{
    comms::task::CONNECTION_STATE,
    dac::dac_task::DAC_HEART_PRESSURE_WATCH,
    heart_control::phase::CardiacPhase,
    valve_task::{PwmValveSetpoint, VALVE_WATCH},
};

/// Pneumatic heart controller routine
#[embassy_executor::task]
pub async fn heart_control_loop(mut setpoint_rx: watch::Receiver<'static, Cs, Setpoint, 3>) {
    info!("starting HEART CONTROL task");

    // let connection_state_rx = CONNECTION_STATE
    //     .receiver()
    //     .expect("Update CONNECTION_STATE N");

    let regulator_pressure_tx = DAC_HEART_PRESSURE_WATCH.sender();
    let valve_tx = VALVE_WATCH.sender();

    info!("HEART CONTROL: Moving mockloop into safe state");
    to_safe_heart_state(&regulator_pressure_tx, &valve_tx);

    info!("HEART CONTROL: starting loop");
    loop {
        trace!("HEART CONTROL: waiting for setpoint");
        if let Some(HeartControllerSetpoint {
            heart_rate,
            pressure,
            systole_ratio,
        }) = setpoint_rx.changed().await.heart_controller_setpoint
        {
            // Heart is enabled: drive regulator and valves
            trace!(
                "HEART CONTROL: Received setpoint ENABLE with {}hz and {}sr",
                heart_rate.get::<hertz>(),
                systole_ratio
            );
            let valve_setpoint = PwmValveSetpoint {
                enable: true,
                frequency: Hertz(heart_rate.get::<hertz>() as u32),
                systole_ratio,
            };

            control_pressure_regulator(pressure, &regulator_pressure_tx);
            control_ventricle_valves(valve_setpoint, &valve_tx);
        } else {
            // Heart is disabled: drive to safe state
            trace!("HEART CONTROL: Received setpoint DISABLE");
            to_safe_heart_state(&regulator_pressure_tx, &valve_tx);
        }
    }
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
    valve_setpoint: PwmValveSetpoint,
    valve_tx: &watch::Sender<'static, Cs, PwmValveSetpoint, 1>,
) {
    valve_tx.send(valve_setpoint);
}

/// Sets the valves and pressure regulator into a safe state
fn to_safe_heart_state(
    heart_pressure_tx: &watch::Sender<'static, Cs, Pressure, 1>,
    valve_tx: &watch::Sender<'static, Cs, PwmValveSetpoint, 1>,
) {
    /// 0 bar pressure seems like the safest state for the solenoid
    const HEART_REGULATOR_SAFE_PRESSURE_BAR: f32 = 0.0;
    /// Safest solenoid state = 0bar pressure. Alternative is Vacuum which seems less safe
    const SAFE_VALVE_SETPOINT: PwmValveSetpoint = PwmValveSetpoint::get_safe();

    trace!("HEART CONTROL: to SAFE state",);

    control_pressure_regulator(
        Pressure::new::<bar>(HEART_REGULATOR_SAFE_PRESSURE_BAR),
        heart_pressure_tx,
    );
    control_ventricle_valves(SAFE_VALVE_SETPOINT, valve_tx);
}

/// Given the current set of measurements and previous state, what is our current state?
fn calculate_appstate() -> AppState {
    AppState::StandBy
}
