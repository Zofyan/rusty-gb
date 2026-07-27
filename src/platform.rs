//! The one place the emulator core reaches for wall-clock time.
//!
//! `dmg` reads `SystemTime`, which does not exist here. The RP2350 has a
//! free-running 64-bit microsecond counter in TIMER0 that serves the same
//! purpose, so both the MBC3 RTC and the frame pacing in [`crate::emulator`]
//! go through this shim rather than naming a platform clock directly.

/// Microseconds since boot.
///
/// TIMER0's low half latches the high half, so `TIMELR` must be read first;
/// reading them in the other order can straddle a wrap and jump by 2^32 us.
///
/// The register pair is read-only and free-running, so stealing the PAC block
/// races with nothing -- there is no state here for a concurrent owner to
/// observe as torn.
#[inline]
pub fn micros() -> u64 {
    let timer = unsafe { &*rp235x_hal::pac::TIMER0::ptr() };
    let low = timer.timelr().read().bits() as u64;
    let high = timer.timehr().read().bits() as u64;
    (high << 32) | low
}

/// Seconds since boot. The MBC3 RTC only needs a monotonic base, not a date.
#[inline]
pub fn secs() -> u64 {
    micros() / 1_000_000
}
