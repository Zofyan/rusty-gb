//! One emulator core, two entry points.
//!
//! * host -- `cargo run`, loads the cartridge from disk and reports peak heap use
//! * Pico 2 -- `cargo run --target thumbv8m.main-none-eabihf --profile embedded`,
//!   runs the cartridge from XIP flash and reports over RTT
//!
//! Everything from [`emulator`] down is target-independent and compiles
//! unchanged for both. The seams are [`platform`] (wall clock), [`output`],
//! [`input`], and the two `main` functions below.
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

extern crate alloc;

mod bus;
mod cpu;
mod emulator;
mod fetcher;
mod input;
mod mbc;
mod memory;
mod output;
mod platform;
mod ppu;
mod register;
mod rom;
#[cfg(target_os = "none")]
mod usb_serial;
mod window_fetcher;

use crate::emulator::Emulator;

// ============================ Pico 2 / RP2350 ============================

#[cfg(target_os = "none")]
mod pico {
    use super::*;
    use crate::rom::Rom;
    use core::mem::MaybeUninit;
    use defmt_rtt as _;
    use embedded_alloc::Heap;
    use panic_halt as _;
    use rp235x_hal as hal;

    #[global_allocator]
    static ALLOCATOR: Heap = Heap::empty();

    #[link_section = ".start_block"]
    #[used]
    pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

    pub(crate) const XTAL_FREQ_HZ: u32 = 12_000_000u32;

    defmt::timestamp!("{=u64:us}", crate::platform::micros());

    /// The cartridge, linked into XIP flash.
    ///
    /// A plain `static` lands in `.rodata`, which on this target is
    /// memory-mapped flash, so [`Rom::mapped`] can address it in place at no
    /// cost in SRAM -- which is the whole reason `Rom` holds `&'static [u8]`
    /// rather than owning a copy.
    static ROM_IMAGE: &[u8] = include_bytes!("../test-roms/Pokemon Red.gb");

    /// Backs the ERAM `Vec` in [`crate::memory::Memory`] plus the one-off USB
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
        let clocks = hal::clocks::init_clocks_and_plls(
            XTAL_FREQ_HZ,
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
        )
        .unwrap();

        // Diagnostics go out over USB CDC rather than defmt/RTT, so that
        // reading them needs no debug probe. From here the stack services
        // itself off USBCTRL_IRQ.
        crate::usb_serial::init(pac.USB, pac.USB_DPRAM, clocks.usb_clock, &mut pac.RESETS);
        crate::usb_serial::wait_for_host();

        let output = crate::output::dummy::Dummy::new();
        let input = crate::input::Dummy::new();

        let mut emu = Emulator::new(Rom::mapped(ROM_IMAGE), input, output);
        emu.run(60 * 10, &mut crate::usb_serial::Serial);

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
    emu.run(60 * 200, &mut Stdout);

    println!("The max amount that was used {}", PEAK_ALLOC.peak_usage_as_kb());
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use crate::emulator::Emulator;
    use crate::input;
    use crate::output::dummy::Dummy;
    use crate::rom::Rom;
    use std::path::Path;

    /// Blargg's cpu_instrs suite. Each ROM reports through the serial port,
    /// which `Emulator::run` drains into the sink passed to it -- so the pass
    /// condition is just what turns up in that string.
    ///
    /// One test per ROM rather than a loop, so a regression names the failing
    /// ROM directly.
    macro_rules! blargg {
        ($name:ident, $rom:literal) => {
            #[test]
            fn $name() {
                let path = Path::new("test-roms")
                    .join("gb-test-roms-master")
                    .join("cpu_instrs")
                    .join("individual")
                    .join($rom);
                let mut emu = Emulator::new(
                    Rom::file(path.to_str().unwrap()),
                    input::Dummy::new(),
                    Dummy::new(),
                );

                let mut serial = String::new();
                emu.run(600, &mut serial);

                assert!(serial.contains("Passed"), "no pass in output: {serial:?}");
                assert!(!serial.contains("Failed"), "failure in output: {serial:?}");
            }
        };
    }

    blargg!(blargg1, "01-special.gb");
    blargg!(blargg2, "02-interrupts.gb");
    blargg!(blargg3, "03-op sp,hl.gb");
    blargg!(blargg4, "04-op r,imm.gb");
    blargg!(blargg5, "05-op rp.gb");
    blargg!(blargg6, "06-ld r,r.gb");
    blargg!(blargg7, "07-jr,jp,call,ret,rst.gb");
    blargg!(blargg8, "08-misc instrs.gb");
    blargg!(blargg9, "09-op r,r.gb");
    blargg!(blargg10, "10-bit ops.gb");
    blargg!(blargg11, "11-op a,(hl).gb");
}
