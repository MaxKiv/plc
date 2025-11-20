use defmt::trace;
use uom::si::{f32::Pressure, pressure::bar};

#[derive(Debug, defmt::Format)]
pub struct RegulatorSetpoint {
    pub pressure: u16,
}

impl RegulatorSetpoint {
    const REGULATOR_MAX_PRESSURE_BAR: f32 = 2.0;
    const REGULATOR_MIN_PRESSURE_BAR: f32 = 0.0;
    const REGULATOR_MAX_VALUE: f32 = ((1 << 13) - 1) as f32;
    const REGULATOR_MIN_VALUE: f32 = 0.0;

    // Convert a given regulator pressure into a DAC Setpoint
    pub fn from_pressure(pressure: Pressure) -> Self {
        let from = pressure.get::<bar>();

        let converted: f32 = (((from - Self::REGULATOR_MIN_PRESSURE_BAR)
            / Self::REGULATOR_MAX_PRESSURE_BAR)
            * Self::REGULATOR_MAX_VALUE)
            .clamp(Self::REGULATOR_MIN_VALUE, Self::REGULATOR_MAX_VALUE);

        let pressure: u16 = converted as u16;
        let setpoint = RegulatorSetpoint { pressure };

        trace!(
            "converted pressure: {:?}bar into DAC setpoint: {:?}",
            from, setpoint,
        );

        setpoint
    }

    // Convert a DAC Setpoint into a regulator pressure
    pub fn to_pressure(self) -> Pressure {
        let converted =
            (self.pressure as f32 / Self::REGULATOR_MAX_VALUE) * Self::REGULATOR_MAX_PRESSURE_BAR;

        let pressure = Pressure::new::<bar>(converted);

        trace!(
            "converted DAC pressure setpoint: {:?} into pressure: {:?}bar",
            self.pressure,
            pressure.get::<bar>()
        );

        pressure
    }
}
