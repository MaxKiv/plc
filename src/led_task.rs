use embassy_executor::task;
use embassy_stm32::gpio::Output;
use embassy_time::Timer;
use embedded_hal::digital::{OutputPin, StatefulOutputPin};

#[derive(PartialEq, Clone, Copy)]
enum AppState {
    StandBy,
    Running,
    Fault,
}

impl AppState {
    fn get_led_blink_period(&self) -> u64 {
        match self {
            AppState::StandBy => 500,
            AppState::Running => 250,
            AppState::Fault => u64::MAX,
        }
    }
}

#[embassy_executor::task]
pub async fn blink_led(mut led: Output<'static>) {
    use AppState::*;

    let led_task_period_ms = 100; // 10Hz
    let mut old_app_state = StandBy;
    let mut new_app_state = Running;
    let mut current_led_period_ms = new_app_state.get_led_blink_period();

    loop {
        if new_app_state != old_app_state {
            current_led_period_ms = 0;
        }

        if current_led_period_ms.saturating_sub(led_task_period_ms) == 0 {
            led.toggle();
            current_led_period_ms = new_app_state.get_led_blink_period();
        }

        old_app_state = new_app_state;

        Timer::after_millis(led_task_period_ms).await;
    }
}
