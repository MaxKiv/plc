use uom::si::{f32::Pressure, pressure::bar};

pub struct ComplianceSetpoint {
    pub pressure: Pressure,
}

impl ComplianceSetpoint {
    pub fn from_raw_compliance(compliance: f32) -> Self {
        defmt::warn!("TODO impl from_raw_compliance");
        let pressure = Pressure::new::<bar>(compliance * 100.0);

        ComplianceSetpoint { pressure }
    }
}
