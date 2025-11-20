pub struct ResistanceSetpoint {
    pub valve_open_percentage: f32,
}

impl ResistanceSetpoint {
    pub fn from_raw_resistance(resistance: f32) -> Self {
        defmt::warn!("TODO impl from_raw_resistance");
        let valve_open_percentage = resistance;

        ResistanceSetpoint {
            valve_open_percentage,
        }
    }
}
