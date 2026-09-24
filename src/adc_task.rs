use defmt::*;
use embassy_stm32::{
    Peri,
    adc::{Adc, AdcChannel, SampleTime},
    peripherals::{ADC1, ADC2, ADC3, DMA1_CH1},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Sender};
use embassy_time::{Duration, Instant, Ticker};
use love_letter::Measurements;
use serde::Serialize;

use crate::hal::{AdcChannels, NUM_INPUTS_ADC1, NUM_INPUTS_ADC2, NUM_INPUTS_ADC3};

const SAMPLE_PERIOD: Duration = Duration::from_millis(10);

static mut DMA_BUF_ADC1: [u16; NUM_INPUTS_ADC1] = [0u16; NUM_INPUTS_ADC1];
static mut DMA_BUF_ADC2: [u16; NUM_INPUTS_ADC2] = [0u16; NUM_INPUTS_ADC2];
static mut DMA_BUF_ADC3: [u16; NUM_INPUTS_ADC3] = [0u16; NUM_INPUTS_ADC3];

#[embassy_executor::task]
pub async fn read_adc(
    mut adc_1: Adc<'static, ADC1>,
    mut adc_2: Adc<'static, ADC2>,
    mut adc_3: Adc<'static, ADC3>,
    mut dma: Peri<'static, DMA1_CH1>,
    adc_channels: AdcChannels,
    frame_out: Sender<'static, Cs, AdcFrame, 2>,
) {
    info!("starting ADC task");

    // Task timekeeper
    let mut ticker = Ticker::every(SAMPLE_PERIOD);
    let read_buffer_adc1 = unsafe { &mut DMA_BUF_ADC1[..] };
    let read_buffer_adc2 = unsafe { &mut DMA_BUF_ADC2[..] };
    let read_buffer_adc3 = unsafe { &mut DMA_BUF_ADC3[..] };

    // Setup ADCS
    let mut heart_actual_pressure = adc_channels.heart_actual_pressure.degrade_adc();
    let mut systemic_compliance_actual_pressure = adc_channels
        .systemic_compliance_actual_pressure
        .degrade_adc();
    let mut pulmonary_compliance_actual_pressure = adc_channels
        .pulmonary_compliance_actual_pressure
        .degrade_adc();
    let mut systemic_flow = adc_channels.systemic_flow.degrade_adc();
    let mut pulmonary_flow = adc_channels.pulmonary_flow.degrade_adc();
    let mut systemic_preload_pressure = adc_channels.systemic_preload_pressure.degrade_adc();
    let mut systemic_afterload_pressure = adc_channels.systemic_afterload_pressure.degrade_adc();
    let mut pulmonary_preload_pressure = adc_channels.pulmonary_preload_pressure.degrade_adc();
    let mut pulmonary_afterload_pressure = adc_channels.pulmonary_afterload_pressure.degrade_adc();

    loop {
        // Read sensor values
        adc_1
            .read(
                dma.reborrow(),
                [
                    (&mut heart_actual_pressure, SampleTime::CYCLES640_5),
                    (
                        &mut systemic_compliance_actual_pressure,
                        SampleTime::CYCLES640_5,
                    ),
                    (
                        &mut pulmonary_compliance_actual_pressure,
                        SampleTime::CYCLES640_5,
                    ),
                    (&mut systemic_flow, SampleTime::CYCLES640_5),
                    (&mut pulmonary_flow, SampleTime::CYCLES640_5),
                ]
                .into_iter(),
                read_buffer_adc1,
            )
            .await;

        adc_2
            .read(
                dma.reborrow(),
                [
                    (&mut systemic_preload_pressure, SampleTime::CYCLES640_5),
                    (&mut systemic_afterload_pressure, SampleTime::CYCLES640_5),
                ]
                .into_iter(),
                read_buffer_adc2,
            )
            .await;

        adc_3
            .read(
                dma.reborrow(),
                [
                    (&mut pulmonary_preload_pressure, SampleTime::CYCLES640_5),
                    (&mut pulmonary_afterload_pressure, SampleTime::CYCLES640_5),
                ]
                .into_iter(),
                read_buffer_adc3,
            )
            .await;

        // Collect into measurement frame
        let timestamp = Instant::now().as_micros();
        let frame = AdcFrame {
            timestamp,
            heart_actual_pressure: read_buffer_adc1[0],
            systemic_compliance_actual_pressure: read_buffer_adc1[1],
            pulmonary_compliance_actual_pressure: read_buffer_adc1[2],
            systemic_flow: read_buffer_adc1[3],
            pulmonary_flow: read_buffer_adc1[4],
            systemic_preload_pressure: read_buffer_adc2[0],
            systemic_afterload_pressure: read_buffer_adc2[1],
            pulmonary_preload_pressure: read_buffer_adc3[0],
            pulmonary_afterload_pressure: read_buffer_adc3[1],
        };
        info!("ADC: {}s measured frame: {:?}", timestamp, frame);

        // Send to anyone interested
        frame_out.send(frame);

        ticker.next().await;
    }
}

#[derive(Format, Serialize, Clone)]
pub struct AdcFrame {
    pub timestamp: u64,
    pub heart_actual_pressure: u16,
    pub systemic_compliance_actual_pressure: u16,
    pub pulmonary_compliance_actual_pressure: u16,
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
            timestamp: self.timestamp,
            heart_actual_pressure: Pressure::new::<millimeter_of_mercury>(
                self.heart_actual_pressure.into(),
            ),
            systemic_compliance_actual_pressure: Pressure::new::<millibar>(
                self.systemic_compliance_actual_pressure.into(),
            ),
            pulmonary_compliance_actual_pressure: Pressure::new::<millibar>(
                self.pulmonary_compliance_actual_pressure.into(),
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
