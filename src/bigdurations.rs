use std::time::Duration;

static MILLIS_PER_MIN: f64 = 60_000f64;
static MILLIS_PER_HOUR: f64 = 3_600_000f64;
static MILLIS_PER_DAY: f64 = 86_400_000f64;
static MILLIS_PER_YEAR: f64 = 31_536_000_000f64;

pub trait BigDurations {
    fn from_mins_f64(mins: f64) -> Self;
    fn from_hours_f64(hours: f64) -> Self;
    fn from_days_f64(days: f64) -> Self;
    fn from_years_f64(years: f64) -> Self;
}

impl BigDurations for Duration {
    fn from_mins_f64(mins: f64) -> Self {
        Self::from_millis((mins * MILLIS_PER_MIN) as u64)
    }

    fn from_hours_f64(hours: f64) -> Self {
        Self::from_millis((hours * MILLIS_PER_HOUR) as u64)
    }

    fn from_days_f64(days: f64) -> Self {
        Self::from_millis((days * MILLIS_PER_DAY) as u64)
    }

    fn from_years_f64(years: f64) -> Self {
        Self::from_millis((years * MILLIS_PER_YEAR) as u64)
    }
}

