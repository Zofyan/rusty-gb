//! Picks the board out of the target triple.
//!
//! Two Picos are supported and they need different things from the linker and
//! from `#[cfg]`:
//!
//! | board | triple | core |
//! | --- | --- | --- |
//! | Pico 1 / W / WH | `thumbv6m-none-eabi` | RP2040, Cortex-M0+ |
//! | Pico 2 | `thumbv8m.main-none-eabihf` | RP2350, Cortex-M33 |
//!
//! Deriving both from `--target` rather than from a feature flag keeps the
//! property the rest of the tree is built around: no build needs a remembered
//! `--features` to be correct, and `cargo test` on the host stays one word long.
//! `target_os = "none"` still separates host from Pico; `rp2040` / `rp2350`
//! separate the two Picos, and nothing outside this file has to parse a triple.
//!
//! The memory layout is the other half. `cortex-m-rt`'s `link.x` does
//! `INCLUDE memory.x`, and `ld` looks for that in the working directory first
//! and then along `-L`. There is deliberately no `memory.x` in the repo root --
//! it would shadow whichever board's file we chose here -- so the two live in
//! `memory/` and the right one is copied into `OUT_DIR` under the name the
//! linker is looking for.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // `cargo::` (two colons) is the current syntax and makes unknown keys an
    // error rather than a silently ignored line.
    println!("cargo::rustc-check-cfg=cfg(rp2040)");
    println!("cargo::rustc-check-cfg=cfg(rp2350)");
    println!("cargo::rerun-if-changed=build.rs");

    let target = env::var("TARGET").expect("cargo sets TARGET");

    let board = match target.as_str() {
        "thumbv6m-none-eabi" => "rp2040",
        "thumbv8m.main-none-eabihf" => "rp2350",
        // Every host target. No linker script, no board cfg -- the emulator
        // core and the test suite build from the same source with neither.
        _ => return,
    };

    println!("cargo::rustc-cfg={board}");

    let layout = PathBuf::from("memory").join(format!("{board}.x"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));

    fs::copy(&layout, out_dir.join("memory.x"))
        .unwrap_or_else(|e| panic!("copying {} into OUT_DIR: {e}", layout.display()));

    // Where `INCLUDE memory.x` resolves from.
    println!("cargo::rustc-link-search={}", out_dir.display());
    println!("cargo::rerun-if-changed={}", layout.display());
}
