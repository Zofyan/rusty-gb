//! The one place the emulator core reaches for wall-clock time.
//!
//! Both the MBC3 RTC in [`crate::mbc`] and the frame pacing in
//! [`crate::emulator`] go through this shim rather than naming a platform clock
//! directly, which is what lets those two modules compile unchanged for the
//! host and for the Pico.

/// Microseconds since an unspecified fixed epoch.
///
/// Only differences between two calls are meaningful, so the epoch differs per
/// target: the Unix epoch on the host, power-on on the Pico.
#[cfg(target_os = "none")]
#[inline]
pub fn micros() -> u64 {
    // TIMER0's low half latches the high half, so `TIMELR` must be read first;
    // reading them in the other order can straddle a wrap and jump by 2^32 us.
    //
    // The register pair is read-only and free-running, so stealing the PAC
    // block races with nothing -- there is no state here for a concurrent owner
    // to observe as torn.
    let timer = unsafe { &*rp235x_hal::pac::TIMER0::ptr() };
    let low = timer.timelr().read().bits() as u64;
    let high = timer.timehr().read().bits() as u64;
    (high << 32) | low
}

#[cfg(not(target_os = "none"))]
#[inline]
pub fn micros() -> u64 {
    // u128 -> u64 cannot truncate for another ~584_000 years.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_micros() as u64
}

/// Seconds on the same base as [`micros`]. The MBC3 RTC only needs a monotonic
/// counter, not a calendar date, so the differing epochs do not matter.
#[inline]
pub fn secs() -> u64 {
    micros() / 1_000_000
}
