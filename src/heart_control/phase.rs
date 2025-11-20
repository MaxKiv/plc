use defmt::debug;
use embassy_time::Duration;
use uom::si::{f32::Frequency, frequency::hertz};

/// Phases of the heart ventricles
/// Systole = ventricle contraction, Diastole = ventricle relaxation
#[derive(Debug, defmt::Format)]
pub enum CardiacPhase {
    Systole,
    Diastole,
}

impl CardiacPhase {
    pub fn switch(self) -> Self {
        match self {
            CardiacPhase::Systole => CardiacPhase::Diastole,
            CardiacPhase::Diastole => CardiacPhase::Systole,
        }
    }

    pub fn get_total_phase_time(&self, heart_rate: Frequency, systole_ratio: f32) -> Duration {
        debug!(
            "getting total phase time for {}hz and {} sys/diastole ratio",
            heart_rate.get::<hertz>(),
            systole_ratio
        );

        const US_IN_SEC: f32 = 1_000_000.0;

        let heart_rate_hz = heart_rate.get::<hertz>();

        let full_cycle_period_us = 1.0 / heart_rate_hz * US_IN_SEC;

        debug!("full_cycle_period_us: {}", full_cycle_period_us);

        let ratio = match self {
            CardiacPhase::Systole => systole_ratio,
            CardiacPhase::Diastole => 1.0 - systole_ratio,
        };

        if full_cycle_period_us == f32::INFINITY {
            debug!("full_cycle_period_us == INF -> {}", Duration::MAX);
            Duration::MAX
        } else {
            let out = (full_cycle_period_us * ratio) as u64;
            debug!("full_cycle_period_us != INF -> {}", out);
            Duration::from_micros(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uom::si::{f32::Frequency, frequency::cycle_per_minute};

    #[test]
    fn test_phase_timing() {
        // Heart rate: 60 bpm = 1 Hz
        let hr = Frequency::new::<cycle_per_minute>(60.0);
        let systole_ratio = 0.3;

        // Full cycle: 1 second
        // Systole: 0.3s
        assert_eq!(
            CardiacPhase::Systole.get_total_phase_time(hr, systole_ratio),
            Duration::from_nanos(300000000)
        );
        // Diastole: 0.7s
        assert_eq!(
            CardiacPhase::Diastole.get_total_phase_time(hr, systole_ratio),
            Duration::from_secs(700000000)
        );

        // More realistic heart rate: 120 bpm = 2 Hz → 0.5s period
        let hr_fast = Frequency::new::<cycle_per_minute>(120.0);

        // Systole: 0.5 * 0.3 = 0.15s
        assert_eq!(
            CardiacPhase::Systole.get_total_phase_time(hr_fast, systole_ratio),
            Duration::from_nanos(150000000)
        );
        // Diastole: 0.5 * 0.7 = 0.35s
        assert_eq!(
            CardiacPhase::Diastole.get_total_phase_time(hr_fast, systole_ratio),
            Duration::from_nanos(350000000)
        );
    }
}
