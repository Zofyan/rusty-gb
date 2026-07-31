//! The emulator core, target-independent.
//!
//! Everything here compiles unchanged for the desktop host and for the Pico 2
//! (RP2350, Cortex-M33). The seams are [`platform`] (wall clock), [`output`],
//! and [`input`]; the two `main` functions and the board glue that only the
//! Pico needs (clocks, USB CDC) live in the binary next door.
//!
//! This is a library rather than a module tree inside `main.rs` so that the
//! integration tests under `tests/` and the benchmarks under `benches/` -- both
//! of which are separate crates -- can link it. `cargo test` exercises the same
//! code the binary does, and `lto = "fat"` in the release profiles means the
//! crate boundary costs the binary nothing.
#![cfg_attr(target_os = "none", no_std)]

extern crate alloc;

pub mod bus;
pub mod cpu;
pub mod emulator;
pub mod fetcher;
pub mod input;
pub mod mbc;
pub mod memory;
pub mod output;
pub mod platform;
pub mod ppu;
pub mod register;
pub mod rom;
/// Host-only: the test-ROM protocols, shared by `tests/`, `benches/` and
/// `examples/`. Gated rather than left in `tests/common/` because all three are
/// separate crates and only a library module can be linked by all of them.
#[cfg(not(target_os = "none"))]
pub mod testrom;
pub mod window_fetcher;
