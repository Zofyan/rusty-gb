//! One emulator core, two entry points.
//!
//! * host -- `cargo run`, loads the cartridge from disk and reports peak heap use
//! * Pico 2 -- `cargo run --target thumbv8m.main-none-eabihf --profile embedded`,
//!   runs the cartridge from XIP flash and reports over USB CDC
//!
//! The core itself is the [`rusty_gb`] library, which is target-independent.
//! What is left here is what cannot be: the two `main` functions, and the board
//! glue below them ([`clocks`], [`usb_serial`]) that only exists on the Pico.
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

extern crate alloc;

#[cfg(target_os = "none")]
mod clocks;
#[cfg(target_os = "none")]
mod usb_serial;

use rusty_gb::emulator::Emulator;
use rusty_gb::{input, output, rom};

// ============================ Pico 2 / RP2350 ============================

#[cfg(target_os = "none")]
mod pico {
    use super::*;
    use core::mem::MaybeUninit;
    use defmt_rtt as _;
    use embedded_alloc::Heap;
    use panic_halt as _;
    use rp235x_hal as hal;
    use rusty_gb::rom::Rom;

    #[global_allocator]
    static ALLOCATOR: Heap = Heap::empty();

    #[link_section = ".start_block"]
    #[used]
    pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

    pub(crate) const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    defmt::timestamp!("{=u64:us}", rusty_gb::platform::micros());

    /// The cartridge, linked into XIP flash.
    ///
    /// A plain `static` lands in `.rodata`, which on this target is
    /// memory-mapped flash, so [`Rom::mapped`] can address it in place at no
    /// cost in SRAM -- which is the whole reason `Rom` holds `&'static [u8]`
    /// rather than owning a copy.
    static ROM_IMAGE: &[u8] = include_bytes!("../test-roms/Pokemon Red.gb");

    /// Backs the ERAM `Vec` in [`rusty_gb::memory::Memory`] plus the one-off USB
    /// bus allocator, so it only has to cover the largest cartridge RAM this
    /// emulator maps (32 KiB, MBC3) with room to spare.
    const HEAP_SIZE: usize = 64 * 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

    #[hal::entry]
    fn main() -> ! {
        // Must precede the first allocation. The previous revision never called
        // this, so the ERAM `Vec` would have faulted on the first cartridge
        // with save RAM.
        unsafe { ALLOCATOR.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }

        let mut pac = hal::pac::Peripherals::take().unwrap();
        let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

        // TIMER0 is deliberately not claimed here: `platform::micros` reads its
        // free-running counter without taking ownership, so the emulator core
        // needs no HAL handle threaded through it.
        //
        // `clocks::init` stands in for `hal::clocks::init_clocks_and_plls`,
        // which brings the part up at its nominal 150 MHz. This is the same
        // sequence with the core voltage and the flash divisor moved first, so
        // that clk_sys can land at `clocks::SYS_MHZ` instead.
        let clocks = crate::clocks::init(
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            pac.POWMAN,
            pac.QMI,
            &mut pac.RESETS,
            &mut watchdog,
        )
        .unwrap();

        // Diagnostics go out over USB CDC rather than defmt/RTT, so that
        // reading them needs no debug probe. From here the stack services
        // itself off USBCTRL_IRQ.
        crate::usb_serial::init(pac.USB, pac.USB_DPRAM, clocks.usb_clock, &mut pac.RESETS);
        crate::usb_serial::wait_for_host();

        // Read back rather than printed from a constant: this is the frequency
        // the PLL actually locked to, so an overclock that silently fell back
        // is visible in the first line out of the port rather than only as
        // disappointing FPS.
        use core::fmt::Write as _;
        use hal::Clock as _;
        let _ = writeln!(
            crate::usb_serial::Serial,
            "rusty-gb: clk_sys {} MHz",
            clocks.system_clock.freq().to_MHz()
        );

        let output = output::dummy::Dummy::new();
        let input = input::Dummy::new();

        // Both sinks are the one CDC port -- `Serial` is a unit struct, so these
        // are two handles to the same device, kept separate only so the Game
        // Boy's serial stream is not interleaved mid-token with diagnostics.
        let mut emu = Emulator::new(Rom::mapped(ROM_IMAGE), input, output);
        emu.run(
            60 * 10,
            &mut crate::usb_serial::Serial,
            &mut crate::usb_serial::Serial,
        );

        // The USB interrupt keeps running, so the port stays open and the final
        // averages remain readable after the run ends.
        loop {
            cortex_m::asm::wfi();
        }
    }
}

// ================================= host =================================

/// Bridges the emulator's [`core::fmt::Write`] sink onto stdout.
///
/// `run` takes `core::fmt::Write` because that is the only writer trait the
/// Pico build has; `io::Stdout` implements `io::Write` instead, so the host
/// needs this adapter rather than passing stdout directly.
#[cfg(not(target_os = "none"))]
struct Stdout;

#[cfg(not(target_os = "none"))]
impl core::fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        use std::io::Write as _;
        let mut out = std::io::stdout();
        out.write_all(s.as_bytes()).expect("Couldn't write");
        // The Game Boy emits serial output a byte at a time and test ROMs are
        // read while they run, so this stays line-unbuffered on purpose.
        out.flush().expect("Couldn't flush");
        Ok(())
    }
}

#[cfg(not(target_os = "none"))]
#[global_allocator]
static PEAK_ALLOC: peak_alloc::PeakAlloc = peak_alloc::PeakAlloc;

#[cfg(not(target_os = "none"))]
fn main() {
    let game = rom::Rom::file("./test-roms/Pokemon Red.gb");
    // Swap for `output::lcd::LCD::new(4)` (windowed) or `output::terminal::…`;
    // both are host-only and gated in `output/mod.rs`.
    let output = output::dummy::Dummy::new();
    let input = input::Dummy::new();

    let mut emu = Emulator::new(game, input, output);
    emu.run(60 * 2000, &mut Stdout, &mut Stdout);

    println!("The max amount that was used {}", PEAK_ALLOC.peak_usage_as_kb());
}
