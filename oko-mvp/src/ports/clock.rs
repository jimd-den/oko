//! Port: clock (SRD §4.3 example ports).

/// A monotonic/wall-clock timestamp in milliseconds.
pub type Timestamp = u64;

pub trait ClockPort: Send + Sync {
    fn now(&self) -> Timestamp;
}
