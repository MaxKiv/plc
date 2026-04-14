use defmt::*;
use embassy_stm32::time::Hertz;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Watch};

use crate::hal::ValvePwm;

pub static VALVE_WATCH: Watch<Cs, PwmValveSetpoint, 1> = Watch::new();

#[derive(Clone, Debug, defmt::Format)]
pub struct PwmValveSetpoint {
    /// Duty cycle percentage
    pub enable: bool,
    pub frequency: f32, // in Hz
    pub systole_ratio: f32,
}

impl PwmValveSetpoint {
    pub const fn get_safe() -> Self {
        Self {
            enable: false,
            frequency: 1.0,
            systole_ratio: love_letter::SYSTOLE_RATIO_DEFAULT,
        }
    }
}

#[embassy_executor::task]
pub async fn control_valves(mut ventricle_pwm: ValvePwm) {
    info!("starting VALVE task");

    let mut rx = VALVE_WATCH.receiver().expect("Increase valve watch size");

    info!("starting VALVE loop");
    loop {
        // Wait for new valve setpoint
        let PwmValveSetpoint {
            enable,
            frequency,
            systole_ratio,
        } = rx.changed().await;

        // Actuate valves according to newly recieved setpoint
        if enable {
            debug!("VALVE ENABLED @ {}hz - {}sr", frequency, systole_ratio);
            ventricle_pwm.enable();
            ventricle_pwm.set_frequency_low(frequency);
            // ventricle_pwm.set_frequency(Hertz(frequency.min(1.0) as u32));
            ventricle_pwm.set_duty(systole_ratio_to_duty_cycle(
                systole_ratio,
                ventricle_pwm.get_max_duty(),
            ));
        } else {
            debug!("VALVE DISABLED");
            ventricle_pwm.disable();
        }
    }
}

fn systole_ratio_to_duty_cycle(systole_ratio: f32, max_duty: u32) -> u32 {
    let dc = (systole_ratio.clamp(0.01, 0.99) * max_duty as f32) as u32;
    debug!("VALVE: systole_ratio {} = duty cycle {}", systole_ratio, dc);
    dc
}
