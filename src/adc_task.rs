use defmt::*;
use embassy_stm32::{
    adc::{self, Adc, SampleTime},
    bind_interrupts,
    peripherals::ADC1,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, channel::Sender};
use embassy_time::Timer;

use crate::{comms::messages::AdcFrame, hal::AdcChannels};

// TODO: do i need this?
// bind_interrupts!(struct Irqs {
//     ADC1_2 => adc::InterruptHandler<ADC1>;
// });

const SAMPLE_PERIOD_MS: u64 = 100;

#[embassy_executor::task]
pub async fn read_adc(
    mut adc: Adc<'static, ADC1>,
    mut adc_channels: AdcChannels,
    frame_out: Sender<'static, Cs, AdcFrame, 2>,
) {
    loop {
        // TODO: use DMA like this
        // let frame = AdcFrame {
        //     regulator_actual_pressure: adc
        //         .read(&mut adc_channels.channel_regulator_actual_pressure)
        //         .await,
        //     systemic_flow: adc.read(&mut adc_channels.channel_systemic_flow).await,
        //     pulmonary_flow: adc.read(&mut adc_channels.channel_systemic_flow).await,
        //     systemic_preload_pressure: adc.read(&mut adc_channels.channel_systemic_flow).await,
        //     systemic_afterload_pressure: adc.read(&mut adc_channels.channel_systemic_flow).await,
        //     pulmonary_preload_pressure: adc.read(&mut adc_channels.channel_systemic_flow).await,
        //     pulmonary_afterload_pressure: adc.read(&mut adc_channels.channel_systemic_flow).await,
        // };

        // Blocking one-shot reads
        let frame = AdcFrame {
            regulator_actual_pressure: adc
                .blocking_read(&mut adc_channels.channel_regulator_actual_pressure),
            systemic_flow: adc.blocking_read(&mut adc_channels.channel_systemic_flow),
            pulmonary_flow: adc.blocking_read(&mut adc_channels.channel_pulmonary_flow),
            systemic_preload_pressure: adc
                .blocking_read(&mut adc_channels.channel_systemic_preload_pressure),
            systemic_afterload_pressure: adc
                .blocking_read(&mut adc_channels.channel_systemic_afterload_pressure),
            pulmonary_preload_pressure: adc
                .blocking_read(&mut adc_channels.channel_pulmonary_preload_pressure),
            pulmonary_afterload_pressure: adc
                .blocking_read(&mut adc_channels.channel_pulmonary_afterload_pressure),
        };

        info!("measured ADC frame: {:?}", frame);

        frame_out.send(frame).await;

        Timer::after_millis(SAMPLE_PERIOD_MS).await;
    }
}
