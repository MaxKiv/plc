use defmt::{debug, info, trace};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch};
use love_letter::Setpoint;
use uom::si::{f32::Pressure, pressure::bar};

use crate::{
    dac::dac_task::{DAC_PULMONARY_COMPLIANCE_WATCH, DAC_SYSTEMIC_COMPLIANCE_WATCH},
    loop_control::setpoint::{compliance::ComplianceSetpoint, resistance::ResistanceSetpoint},
};

/// Mockloop control loop
/// This control mockloop parameters like systemic/pulmonary flow resistance and compliance
#[embassy_executor::task]
pub async fn mockloop_control_loop(mut setpoint_rx: watch::Receiver<'static, Cs, Setpoint, 3>) {
    info!("starting LOOP CONTROL task");

    // let connection_state_rx = CONNECTION_STATE
    //     .receiver()
    //     .expect("Update CONNECTION_STATE N");

    let systemic_pressure_tx = DAC_SYSTEMIC_COMPLIANCE_WATCH.sender();
    let pulmonary_pressure_tx = DAC_PULMONARY_COMPLIANCE_WATCH.sender();

    info!("LOOP CONTROL: Moving mockloop into safe state");
    to_safe_loop_state(&systemic_pressure_tx, &pulmonary_pressure_tx);

    info!("LOOP CONTROL: Waiting for initial setpoint");
    // Current setpoint
    let mut setpoint = setpoint_rx.changed().await;

    info!("LOOP CONTROL: starting loop");
    loop {
        // Only control the mockloop if the loop controller is enabled
        if let Some(ref mockloop_setpoint) = setpoint.mockloop_setpoint {
            // Convert raw compliance setpoint into pressure setpoint for the compliance chamber
            // pressure regulators
            let pulmonary_pressure_setpoint = ComplianceSetpoint::from_raw_compliance(
                mockloop_setpoint.systemic_afterload_compliance,
            );
            let systemic_pressure_setpoint = ComplianceSetpoint::from_raw_compliance(
                mockloop_setpoint.systemic_afterload_compliance,
            );

            debug!(
                "LOOP CONTROL: Converted raw systemic compliance setpoint {} into pressure setpoint {}bar",
                mockloop_setpoint.systemic_afterload_compliance,
                systemic_pressure_setpoint.pressure.get::<bar>()
            );
            debug!(
                "LOOP CONTROL: Converted raw pulmonary compliance setpoint {} into pressure setpoint {}bar",
                mockloop_setpoint.pulmonary_afterload_compliance,
                pulmonary_pressure_setpoint.pressure.get::<bar>()
            );

            let systemic_resistance_setpoint =
                ResistanceSetpoint::from_raw_resistance(mockloop_setpoint.systemic_resistance);
            debug!(
                "LOOP CONTROL: Converted raw systemic resistance setpoint {} into setpoint {}",
                mockloop_setpoint.systemic_resistance,
                systemic_resistance_setpoint.valve_open_percentage
            );
            let pulmonary_resistance_setpoint =
                ResistanceSetpoint::from_raw_resistance(mockloop_setpoint.pulmonary_resistance);
            debug!(
                "LOOP CONTROL: Converted raw pulmonary resistance setpoint {} into setpoint {}",
                mockloop_setpoint.pulmonary_resistance,
                pulmonary_resistance_setpoint.valve_open_percentage
            );

            // Ask DAC task to actuate the compliance chamber regulators
            systemic_pressure_tx.send(systemic_pressure_setpoint.pressure);
            pulmonary_pressure_tx.send(pulmonary_pressure_setpoint.pressure);

            // TODO: Control resistance
        } else {
            // Heart Controller is disabled: Set the valves and pressure regulator into safe state
            debug!("LOOP CONTROL: DISABLED -> Moving to safe state and ready for more action");

            to_safe_loop_state(&systemic_pressure_tx, &pulmonary_pressure_tx);
        }

        // Await a new setpoint
        setpoint = setpoint_rx.changed().await;
    }
}

/// Sets the valves and pressure regulator into a safe state
fn to_safe_loop_state(
    systemic_pressure_tx: &watch::Sender<'static, Cs, Pressure, 1>,
    pulmonary_pressure_tx: &watch::Sender<'static, Cs, Pressure, 1>,
) {
    const COMPLIANCE_REGULATOR_SAFE_PRESSURE_BAR: f32 = 0.0;

    debug!("HEART CONTROL: to SAFE state",);

    systemic_pressure_tx.send(Pressure::new::<bar>(COMPLIANCE_REGULATOR_SAFE_PRESSURE_BAR));
    pulmonary_pressure_tx.send(Pressure::new::<bar>(COMPLIANCE_REGULATOR_SAFE_PRESSURE_BAR));
}
