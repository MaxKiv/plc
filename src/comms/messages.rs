use crate::Measurements;
use defmt::Format;
use serde::Serialize;

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

impl AdcFrame {
    /// Convert an adc frame to si units and collect into a measurement set
    pub fn into_measurement(self) -> Measurements {
        use uom::si::pressure::*;
        use uom::si::volume_rate::*;

        Measurements {
            regulator_actual_pressure: Pressure::new::<millimeter_of_mercury>(
                self.regulator_actual_pressure.into(),
            ),
            systemic_flow: VolumeRate::new::<liter_per_minute>(self.systemic_flow.into()),
            pulmonary_flow: VolumeRate::new::<liter_per_minute>(self.pulmonary_flow.into()),
            systemic_preload_pressure: Pressure::new::<millimeter_of_mercury>(
                self.systemic_preload_pressure.into(),
            ),
            systemic_afterload_pressure: Pressure::new::<millimeter_of_mercury>(
                self.systemic_afterload_pressure.into(),
            ),
            pulmonary_preload_pressure: Pressure::new::<millimeter_of_mercury>(
                self.pulmonary_preload_pressure.into(),
            ),
            pulmonary_afterload_pressure: Pressure::new::<millimeter_of_mercury>(
                self.pulmonary_afterload_pressure.into(),
            ),
        }
    }
}
