//! Pico clock bring-up, above each part's nominal system clock.
//!
//! Each HAL ships a one-call `init_clocks_and_plls` that brings its part up at
//! the datasheet speed -- 150 MHz on the RP2350, 125 MHz on the RP2040. The
//! emulator is CPU-bound on both boards, so the clock is where the next
//! doubling comes from, and the reason these modules exist rather than a
//! one-line change to the PLL config is the ordering around it: the core
//! voltage has to rise *before* the frequency does, or the first clock edge at
//! the new speed lands on a core still at its default voltage.
//!
//! Both modules expose the same two items, so [`crate::pico`] needs no `cfg` to
//! read the clock back or to print it:
//!
//! * `SYS_MHZ` -- the one knob. Set it to the part's stock speed and everything
//!   downstream (PLL config, core voltage, flash timing) follows it back to
//!   default.
//! * `init` -- the HAL's `init_clocks_and_plls` plus whatever that part needs
//!   moved first. The peripherals differ, so the two signatures do too.
//!
//! **Both defaults are out of spec.** Silicon varies, and a part that will not
//! hold the clock configured here shows it as a hang or a corrupted frame, not
//! as an error message. The first line out of the USB port is the frequency the
//! PLL actually locked to, so a fallback is visible immediately; if the board
//! is unstable, drop `SYS_MHZ` in the module for your board.

#[cfg(rp2040)]
mod rp2040;
#[cfg(rp2350)]
mod rp2350;

#[cfg(rp2040)]
pub use rp2040::{init, SYS_MHZ};
#[cfg(rp2350)]
pub use rp2350::{init, SYS_MHZ};
