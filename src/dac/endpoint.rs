use defmt::info;
use embassy_stm32::{dac, mode::Async};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch};
use uom::si::{f32::Pressure, pressure::bar};

use crate::dac::setpoint::RegulatorSetpoint;

pub struct DacEndpoint<T: embassy_stm32::dac::Instance, C: embassy_stm32::dac::Channel + 'static> {
    pub id: DacId,
    pub dac: embassy_stm32::dac::DacChannel<'static, T, C, Async>,
    pub rx: watch::Receiver<'static, Cs, Pressure, 1>,
}

#[derive(defmt::Format)]
pub enum DacId {
    Heart,
    Systemic,
    Pulmonary,
}

pub async fn handle_endpoint<T, C>(endpoint: &mut DacEndpoint<T, C>)
where
    T: dac::Instance,
    C: dac::Channel,
{
    let setpoint = endpoint.rx.changed().await;

    info!(
        "DAC: setting {:?} pressure to {:?}bar",
        endpoint.id,
        setpoint.get::<bar>()
    );

    let setpoint = RegulatorSetpoint::from_pressure(setpoint);

    endpoint
        .dac
        .set(embassy_stm32::dac::Value::Bit12Right(setpoint.pressure));
}
