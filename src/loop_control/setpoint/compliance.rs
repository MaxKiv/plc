use uom::si::{f32::Pressure, pressure::millibar};

pub struct ComplianceSetpoint {
    pub pressure: Pressure,
}

impl ComplianceSetpoint {
    pub fn from_pressure_setpoint(pressure: Pressure) -> Self {
        defmt::warn!("TODO impl from_raw_compliance");

        ComplianceSetpoint { pressure }
    }
}
