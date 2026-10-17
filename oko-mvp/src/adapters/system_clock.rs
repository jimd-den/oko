//! Adapter: system clock (SRD §4.3 example ports).

use crate::ports::clock::{ClockPort, Timestamp};
use std::time::SystemTime;

pub struct SystemClockAdapter;

impl Default for SystemClockAdapter {
    fn default() -> Self {
        Self
    }
}

impl SystemClockAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl ClockPort for SystemClockAdapter {
    fn now(&self) -> Timestamp {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
