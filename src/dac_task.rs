use defmt::*;
use embassy_stm32::{
    dac::{Ch1, DacChannel},
    mode::Async,
    peripherals::DAC1,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, signal::Signal, watch::Watch};
use uom::si::{f32::Pressure, pressure::bar};

pub static DAC_REGULATOR_PRESSURE_WATCH: Watch<Cs, Pressure, 1> = Watch::new();

#[embassy_executor::task]
pub async fn write_dac(mut pressure_regulator_dac: DacChannel<'static, DAC1, Ch1, Async>) {
    info!("starting DAC task");

    let mut rx = DAC_REGULATOR_PRESSURE_WATCH
        .receiver()
        .expect("increase pressure reg watch size");

    info!("starting DAC loop");
    loop {
        let pressure_setpoint = rx.changed().await;

        info!(
            "DAC: setting regulator pressure to {:?}bar",
            pressure_setpoint.get::<bar>()
        );
        pressure_regulator_dac.set(embassy_stm32::dac::Value::Bit12Right(
            RegulatorSetpoint::from_pressure(pressure_setpoint).pressure,
        ));
    }
}

#[derive(Debug, defmt::Format)]
pub struct RegulatorSetpoint {
    pressure: u16,
}

impl RegulatorSetpoint {
    const REGULATOR_MAX_PRESSURE_BAR: f32 = 2.0;
    const REGULATOR_MIN_PRESSURE_BAR: f32 = 0.0;
    const REGULATOR_MAX_VALUE: f32 = ((1 << 13) - 1) as f32;
    const REGULATOR_MIN_VALUE: f32 = 0.0;

    // Convert a given regulator pressure into a DAC Setpoint
    fn from_pressure(pressure: Pressure) -> Self {
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
    fn to_pressure(self) -> Pressure {
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
