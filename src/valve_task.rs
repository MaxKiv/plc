use defmt::*;
use embassy_stm32::peripherals::TIM1;
use embassy_stm32::timer::Channel;
use embassy_stm32::{time::Hertz, timer::complementary_pwm::ComplementaryPwm};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex as Cs, watch::Watch};

use crate::hal::ValvePwm;

pub static VALVE_WATCH: Watch<Cs, PwmValveSetpoint, 1> = Watch::new();

#[derive(Clone, Debug, defmt::Format)]
pub struct PwmValveSetpoint {
    /// Duty cycle percentage
    pub enable: bool,
    pub frequency: Hertz,
    pub systole_ratio: f32,
}

impl PwmValveSetpoint {
    pub const fn get_safe() -> Self {
        Self {
            enable: false,
            frequency: Hertz(1),
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
            trace!("VALVE ENABLED @ {}hz - {}sr", frequency, systole_ratio);
            ventricle_pwm.enable();
            ventricle_pwm.set_frequency(frequency);
            ventricle_pwm.set_duty(systole_ratio_to_duty_cycle(
                systole_ratio,
                ventricle_pwm.get_max_duty(),
            ));
        } else {
            trace!("VALVE DISABLED");
            ventricle_pwm.disable();
        }
    }
}

fn systole_ratio_to_duty_cycle(systole_ratio: f32, max_duty: u16) -> u16 {
    (systole_ratio.clamp(0.0, 1.0) * max_duty as f32) as u16
}
