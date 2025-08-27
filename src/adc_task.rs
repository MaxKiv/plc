use defmt::*;
use embassy_stm32::{
    Peri,
    adc::{Adc, AdcChannel, SampleTime},
    peripherals::{ADC1, DMA1_CH1},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, channel::Sender};
use embassy_time::{Duration, Instant, Timer};
use love_letter::Measurements;
use serde::Serialize;

use crate::hal::{AdcChannels, NUM_ADC_INPUTS};

const SAMPLE_PERIOD: Duration = Duration::from_millis(100);

static mut DMA_BUF: [u16; NUM_ADC_INPUTS] = [0u16; NUM_ADC_INPUTS];

#[embassy_executor::task]
pub async fn read_adc(
    mut adc: Adc<'static, ADC1>,
    mut dma: Peri<'static, DMA1_CH1>,
    adc_channels: AdcChannels,
    frame_out: Sender<'static, Cs, AdcFrame, 2>,
) {
    info!("starting ADC task");

    let mut read_buffer = unsafe { &mut DMA_BUF[..] };

    let mut regulator_pressure = adc_channels.regulator_actual_pressure.degrade_adc();
    let mut systemic_flow = adc_channels.systemic_flow.degrade_adc();
    let mut pulmonary_flow = adc_channels.pulmonary_flow.degrade_adc();
    let mut systemic_preload_pressure = adc_channels.systemic_preload_pressure.degrade_adc();
    let mut systemic_afterload_pressure = adc_channels.systemic_afterload_pressure.degrade_adc();
    let mut pulmonary_preload_pressure = adc_channels.pulmonary_preload_pressure.degrade_adc();
    let mut pulmonary_afterload_pressure = adc_channels.pulmonary_afterload_pressure.degrade_adc();

    loop {
        adc.read(
            dma.reborrow(),
            [
                (&mut regulator_pressure, SampleTime::CYCLES24_5),
                (&mut systemic_flow, SampleTime::CYCLES24_5),
                (&mut pulmonary_flow, SampleTime::CYCLES24_5),
                (&mut systemic_preload_pressure, SampleTime::CYCLES24_5),
                (&mut systemic_afterload_pressure, SampleTime::CYCLES24_5),
                (&mut pulmonary_preload_pressure, SampleTime::CYCLES24_5),
                (&mut pulmonary_afterload_pressure, SampleTime::CYCLES24_5),
            ]
            .into_iter(),
            &mut read_buffer,
        )
        .await;

        let frame = AdcFrame {
            regulator_actual_pressure: read_buffer[0],
            systemic_flow: read_buffer[1],
            pulmonary_flow: read_buffer[2],
            systemic_preload_pressure: read_buffer[3],
            systemic_afterload_pressure: read_buffer[4],
            pulmonary_preload_pressure: read_buffer[5],
            pulmonary_afterload_pressure: read_buffer[6],
        };

        info!("ADC: measured frame: {:?}", frame);

        frame_out.send(frame).await;

        Timer::after(SAMPLE_PERIOD).await;
    }
}

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
            timestamp: Instant::now().as_micros(),
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
