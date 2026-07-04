//! Small `no_std` time helpers for platform-driven code.
//!
//! `core` has task polling primitives, but it does not provide a clock or a
//! duration type. Shell Vessel keeps that boundary explicit: your platform owns
//! the hardware clock, and this module gives the rest of the crate a tiny,
//! millisecond-based shape to talk to.
//!
//! The usual platform loop is:
//!
//! 1. read the platform clock
//! 2. convert it to [`Instant`]
//! 3. advance the code that needs time
//! 4. sleep, wait for an interrupt, or poll again
//!
//! A platform adapter is intentionally small:
//!
//! ```ignore
//! use shvessel::time::{Clock, Instant};
//!
//! struct EspClock;
//!
//! impl Clock for EspClock {
//!     fn now(&mut self) -> Instant {
//!         Instant::from_millis(esp_hal::time::Instant::now()
//!             .duration_since_epoch()
//!             .as_millis())
//!     }
//! }
//! ```

use core::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Duration {
    millis: u64,
}

impl Duration {
    pub const ZERO: Self = Self::from_millis(0);

    pub const fn from_millis(millis: u64) -> Self {
        Self { millis }
    }

    pub const fn as_millis(self) -> u64 {
        self.millis
    }

    pub const fn is_zero(self) -> bool {
        self.millis == 0
    }

    pub const fn saturating_add(self, other: Self) -> Self {
        Self::from_millis(self.millis.saturating_add(other.millis))
    }

    pub const fn saturating_sub(self, other: Self) -> Self {
        Self::from_millis(self.millis.saturating_sub(other.millis))
    }

    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.millis.checked_add(other.millis) {
            Some(millis) => Some(Self::from_millis(millis)),
            None => None,
        }
    }

    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.millis.checked_sub(other.millis) {
            Some(millis) => Some(Self::from_millis(millis)),
            None => None,
        }
    }
}

impl From<u64> for Duration {
    fn from(millis: u64) -> Self {
        Self::from_millis(millis)
    }
}

impl From<Duration> for u64 {
    fn from(duration: Duration) -> Self {
        duration.as_millis()
    }
}

impl Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.saturating_add(rhs)
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.saturating_add(rhs);
    }
}

impl Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.saturating_sub(rhs)
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.saturating_sub(rhs);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Instant {
    millis: u64,
}

impl Instant {
    pub const ZERO: Self = Self::from_millis(0);

    pub const fn from_millis(millis: u64) -> Self {
        Self { millis }
    }

    pub const fn as_millis(self) -> u64 {
        self.millis
    }

    pub const fn saturating_duration_since(self, earlier: Self) -> Duration {
        Duration::from_millis(self.millis.saturating_sub(earlier.millis))
    }

    pub const fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        match self.millis.checked_sub(earlier.millis) {
            Some(millis) => Some(Duration::from_millis(millis)),
            None => None,
        }
    }

    pub const fn duration_until(self, deadline: Self) -> Duration {
        deadline.saturating_duration_since(self)
    }

    pub const fn saturating_add(self, duration: Duration) -> Self {
        Self::from_millis(self.millis.saturating_add(duration.as_millis()))
    }

    pub const fn saturating_sub(self, duration: Duration) -> Self {
        Self::from_millis(self.millis.saturating_sub(duration.as_millis()))
    }

    pub const fn checked_add(self, duration: Duration) -> Option<Self> {
        match self.millis.checked_add(duration.as_millis()) {
            Some(millis) => Some(Self::from_millis(millis)),
            None => None,
        }
    }

    pub const fn checked_sub(self, duration: Duration) -> Option<Self> {
        match self.millis.checked_sub(duration.as_millis()) {
            Some(millis) => Some(Self::from_millis(millis)),
            None => None,
        }
    }
}

impl From<u64> for Instant {
    fn from(millis: u64) -> Self {
        Self::from_millis(millis)
    }
}

impl From<Instant> for u64 {
    fn from(instant: Instant) -> Self {
        instant.as_millis()
    }
}

impl Add<Duration> for Instant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        self.saturating_add(rhs)
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = self.saturating_add(rhs);
    }
}

impl Sub<Duration> for Instant {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.saturating_sub(rhs)
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = self.saturating_sub(rhs);
    }
}

impl Sub for Instant {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.saturating_duration_since(rhs)
    }
}

pub trait Clock {
    fn now(&mut self) -> Instant;

    fn now_ms(&mut self) -> u64 {
        self.now().as_millis()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ManualClock {
    now: Instant,
}

impl ManualClock {
    pub const fn new(now: Instant) -> Self {
        Self { now }
    }

    pub const fn zero() -> Self {
        Self::new(Instant::ZERO)
    }

    pub const fn current(self) -> Instant {
        self.now
    }

    pub const fn current_ms(self) -> u64 {
        self.now.as_millis()
    }

    pub fn set(&mut self, now: Instant) {
        self.now = now;
    }

    pub fn advance(&mut self, duration: Duration) {
        self.now += duration;
    }

    pub fn advance_millis(&mut self, millis: u64) {
        self.advance(Duration::from_millis(millis));
    }
}

impl Clock for ManualClock {
    fn now(&mut self) -> Instant {
        self.now
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, Duration, Instant, ManualClock};

    #[test]
    fn instant_and_duration_use_saturating_math() {
        let start = Instant::from_millis(10);
        let later = start + Duration::from_millis(25);

        assert_eq!(later.as_millis(), 35);
        assert_eq!((later - start).as_millis(), 25);
        assert_eq!((start - later).as_millis(), 0);
        assert_eq!(start.duration_until(later).as_millis(), 25);
    }

    #[test]
    fn manual_clock_can_be_driven_by_tests_or_platform_shims() {
        let mut clock = ManualClock::zero();

        assert_eq!(clock.now_ms(), 0);
        clock.advance_millis(5);
        assert_eq!(clock.now(), Instant::from_millis(5));
        clock.set(Instant::from_millis(42));
        assert_eq!(clock.now_ms(), 42);
    }
}
