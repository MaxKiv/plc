use defmt::{debug, info};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch};
use love_letter::{MockloopSetpoint, Setpoint};
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

    let systemic_pressure_tx = DAC_SYSTEMIC_COMPLIANCE_WATCH.sender();
    let pulmonary_pressure_tx = DAC_PULMONARY_COMPLIANCE_WATCH.sender();

    info!("LOOP CONTROL: Moving mockloop into safe state");
    to_safe_loop_state(&systemic_pressure_tx, &pulmonary_pressure_tx);

    info!("LOOP CONTROL: starting loop");
    loop {
        debug!("LOOP CONTROL: waiting for setpoint");

        let setpoint = setpoint_rx.changed().await;

        // Destructure
        let MockloopSetpoint {
            enable,
            systemic_resistance,
            pulmonary_resistance,
            systemic_afterload_compliance,
            pulmonary_afterload_compliance,
        } = setpoint.mockloop_setpoint;

        // Only control the mockloop if the loop controller is enabled
        if enable {
            // Convert raw compliance setpoint into pressure setpoint for the compliance chamber
            // pressure regulators
            let pulmonary_pressure_setpoint =
                ComplianceSetpoint::from_raw_compliance(pulmonary_afterload_compliance);
            let systemic_pressure_setpoint =
                ComplianceSetpoint::from_raw_compliance(systemic_afterload_compliance);

            debug!(
                "LOOP CONTROL: ENABLED -> Converted raw systemic compliance setpoint {} into pressure setpoint {}bar",
                systemic_afterload_compliance,
                systemic_pressure_setpoint.pressure.get::<bar>()
            );
            debug!(
                "LOOP CONTROL: ENABLED -> Converted raw pulmonary compliance setpoint {} into pressure setpoint {}bar",
                pulmonary_afterload_compliance,
                pulmonary_pressure_setpoint.pressure.get::<bar>()
            );

            let systemic_resistance_setpoint =
                ResistanceSetpoint::from_raw_resistance(systemic_resistance);
            debug!(
                "LOOP CONTROL: ENABLED -> Converted raw systemic resistance setpoint {} into setpoint {}",
                systemic_resistance, systemic_resistance_setpoint.valve_open_percentage
            );
            let pulmonary_resistance_setpoint =
                ResistanceSetpoint::from_raw_resistance(pulmonary_resistance);
            debug!(
                "LOOP CONTROL: ENABLED -> Converted raw pulmonary resistance setpoint {} into setpoint {}",
                pulmonary_resistance, pulmonary_resistance_setpoint.valve_open_percentage
            );

            // Ask DAC task to actuate the compliance chamber regulators
            systemic_pressure_tx.send(systemic_pressure_setpoint.pressure);
            pulmonary_pressure_tx.send(pulmonary_pressure_setpoint.pressure);

            // TODO: Control resistance
        } else {
            // Loop Controller is disabled: Set the valves and pressure regulator into safe state
            debug!("LOOP CONTROL: DISABLED -> Moving to safe state and ready for more action");

            to_safe_loop_state(&systemic_pressure_tx, &pulmonary_pressure_tx);
        }
    }
}

/// Sets the valves and pressure regulator into a safe state
fn to_safe_loop_state(
    systemic_pressure_tx: &watch::Sender<'static, Cs, Pressure, 1>,
    pulmonary_pressure_tx: &watch::Sender<'static, Cs, Pressure, 1>,
) {
    const COMPLIANCE_REGULATOR_SAFE_PRESSURE_BAR: f32 = 0.0;

    debug!("LOOP CONTROL: to SAFE state",);

    systemic_pressure_tx.send(Pressure::new::<bar>(COMPLIANCE_REGULATOR_SAFE_PRESSURE_BAR));
    pulmonary_pressure_tx.send(Pressure::new::<bar>(COMPLIANCE_REGULATOR_SAFE_PRESSURE_BAR));
}
