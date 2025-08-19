use defmt::Format;
use serde::{Deserialize, Serialize};
use uom::si::u32::{Pressure, VolumeRate};

use crate::AppState;

#[derive(Format, Serialize)]
pub struct AdcFrame {
    pub regulator_actual_pressure: u16,
    pub systemic_flow: u16,
    pub pulmonary_flow: u16,
    pub systemic_preload_pressure: u16,
    pub systemic_afterload_pressure: u16,
    pub pulmonary_preload_pressure: u16,
    pub pulmonary_afterload_pressure: u16,
}

#[derive(Serialize, Clone)]
pub struct Measurements {
    regulator_actual_pressure: Pressure,
    systemic_flow: VolumeRate,
    pulmonary_flow: VolumeRate,
    systemic_preload_pressure: Pressure,
    systemic_afterload_pressure: Pressure,
    pulmonary_preload_pressure: Pressure,
    pulmonary_afterload_pressure: Pressure,
}

impl Format for Measurements {
    fn format(&self, fmt: defmt::Formatter) {
        use uom::si::pressure::millimeter_of_mercury;
        use uom::si::volume_rate::liter_per_minute;

        defmt::write!(
            fmt,
            "Measurement(reg: {} mmHg, sf: {} lpm, pf: {} lpm, spp: {} mmHg, sap: {} mmHg, ppp: {} mmHg, pap: {} mmHg",
            self.regulator_actual_pressure
                .get::<millimeter_of_mercury>(),
            self.systemic_flow.get::<liter_per_minute>(),
            self.systemic_flow.get::<liter_per_minute>(),
            self.systemic_preload_pressure
                .get::<millimeter_of_mercury>(),
            self.systemic_afterload_pressure
                .get::<millimeter_of_mercury>(),
            self.pulmonary_preload_pressure
                .get::<millimeter_of_mercury>(),
            self.pulmonary_afterload_pressure
                .get::<millimeter_of_mercury>(),
        );
    }
}

impl Measurements {
    /// Convert an adc frame to si units and collect into a measurement set
    pub fn from_frame(frame: AdcFrame) -> Self {
        use uom::si::pressure::*;
        use uom::si::volume_rate::*;

        Self {
            regulator_actual_pressure: Pressure::new::<millimeter_of_mercury>(
                frame.regulator_actual_pressure.into(),
            ),
            systemic_flow: VolumeRate::new::<liter_per_minute>(frame.systemic_flow.into()),
            pulmonary_flow: VolumeRate::new::<liter_per_minute>(frame.pulmonary_flow.into()),
            systemic_preload_pressure: Pressure::new::<millimeter_of_mercury>(
                frame.systemic_preload_pressure.into(),
            ),
            systemic_afterload_pressure: Pressure::new::<millimeter_of_mercury>(
                frame.systemic_afterload_pressure.into(),
            ),
            pulmonary_preload_pressure: Pressure::new::<millimeter_of_mercury>(
                frame.pulmonary_preload_pressure.into(),
            ),
            pulmonary_afterload_pressure: Pressure::new::<millimeter_of_mercury>(
                frame.pulmonary_afterload_pressure.into(),
            ),
        }
    }
}

#[derive(Serialize, Clone, Format)]
pub struct Report {
    pub app_state: AppState,
    pub measurements: Measurements,
}

#[derive(Deserialize, Clone)]
pub struct Setpoint {
    pressure: Pressure,
    e_valve: bool,
    heart_valve_right: bool,
    heart_valve_left: bool,
}

impl Format for Setpoint {
    fn format(&self, fmt: defmt::Formatter) {
        use uom::si::pressure::millibar;

        defmt::write!(
            fmt,
            "Setpoint(regulator: {} mbar, e_valve: {}, heart_valve_left: {}, heart_valve_right: {}",
            self.pressure.get::<millibar>(),
            self.e_valve,
            self.heart_valve_left,
            self.heart_valve_left,
        );
    }
}
