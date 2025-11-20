use defmt::*;
use embassy_futures::select::select3;
use embassy_stm32::{
    dac::{Ch1, Ch2, DacChannel},
    mode::Async,
    peripherals::{DAC1, DAC2},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Watch};
use uom::si::f32::Pressure;

use crate::dac::endpoint::{DacEndpoint, DacId, handle_endpoint};

pub static DAC_HEART_PRESSURE_WATCH: Watch<Cs, Pressure, 1> = Watch::new();
pub static DAC_SYSTEMIC_COMPLIANCE_WATCH: Watch<Cs, Pressure, 1> = Watch::new();
pub static DAC_PULMONARY_COMPLIANCE_WATCH: Watch<Cs, Pressure, 1> = Watch::new();

#[embassy_executor::task]
pub async fn write_dac(
    heart_pressure_dac: DacChannel<'static, DAC1, Ch1, Async>,
    systemic_compliance_dac: DacChannel<'static, DAC1, Ch2, Async>,
    pulmonary_compliance_dac: DacChannel<'static, DAC2, Ch1, Async>,
) {
    info!("starting DAC task");

    let mut heart_endpoint = DacEndpoint {
        id: DacId::Heart,
        dac: heart_pressure_dac,
        rx: DAC_HEART_PRESSURE_WATCH
            .receiver()
            .expect("increase heart pressure N"),
    };

    let mut systemic_endpoint = DacEndpoint {
        id: DacId::Systemic,
        dac: systemic_compliance_dac,
        rx: DAC_SYSTEMIC_COMPLIANCE_WATCH
            .receiver()
            .expect("increase systemic compliance pressure N"),
    };

    let mut pulmonary_endpoint = DacEndpoint {
        id: DacId::Pulmonary,
        dac: pulmonary_compliance_dac,
        rx: DAC_PULMONARY_COMPLIANCE_WATCH
            .receiver()
            .expect("increase pulmonary compliance pressure N"),
    };

    info!("starting DAC loop");
    loop {
        select3(
            handle_endpoint(&mut heart_endpoint),
            handle_endpoint(&mut systemic_endpoint),
            handle_endpoint(&mut pulmonary_endpoint),
        )
        .await;
    }
}
