use serde::{Deserialize, Serialize};
use uom::si::u32::{Pressure, VolumeRate};

use crate::AppState;

#[derive(defmt::Format, Serialize)]
pub struct AdcFrame {
    pub regulator_actual_pressure: u16,
    pub systemic_flow: u16,
    pub pulmonary_flow: u16,
    pub systemic_preload_pressure: u16,
    pub systemic_afterload_pressure: u16,
    pub pulmonary_preload_pressure: u16,
    pub pulmonary_afterload_pressure: u16,
}

#[derive(Serialize)]
pub struct Measurements {
    regulator_actual_pressure: Pressure,
    systemic_flow: VolumeRate,
    pulmonary_flow: VolumeRate,
    systemic_preload_pressure: Pressure,
    systemic_afterload_pressure: Pressure,
    pulmonary_preload_pressure: Pressure,
    pulmonary_afterload_pressure: Pressure,
}

#[derive(Serialize)]
pub struct Report {
    app_state: AppState,
    measurements: Measurements,
}

#[derive(Deserialize)]
pub struct Setpoint {
    pressure: Pressure,
    e_valve: bool,
    heart_valve_right: bool,
    heart_valve_left: bool,
}
