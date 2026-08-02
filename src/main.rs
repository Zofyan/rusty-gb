//! One emulator core, two entry points.
//!
//! * host -- `cargo run`, loads the cartridge from disk and reports peak heap use
//! * Pico -- `cargo run --target <triple> --profile embedded`, runs the
//!   cartridge from XIP flash and reports over USB CDC
//!
//! Two boards share that second entry point, and `--target` is all that
//! chooses between them:
//!
//! | board | triple | core |
//! | --- | --- | --- |
//! | Pico 1 / W / WH | `thumbv6m-none-eabi` | RP2040, Cortex-M0+ |
//! | Pico 2 | `thumbv8m.main-none-eabihf` | RP2350, Cortex-M33 |
//!
//! `build.rs` turns the triple into the `rp2040` / `rp2350` cfgs used below and
//! picks the matching linker script out of `memory/`.
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

// ======================= Pico 1 / RP2040, Pico 2 / RP2350 =======================

#[cfg(target_os = "none")]
mod pico {
    use super::*;
    use core::mem::MaybeUninit;
    use defmt_rtt as _;
    use embedded_alloc::Heap;
    use panic_halt as _;
    use rusty_gb::hal;
    use rusty_gb::rom::Rom;
    use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
    use embedded_hal_bus::spi::ExclusiveDevice;
    use mipidsi::{interface::SpiInterface, models::ILI9225Rgb565, Builder};
    use static_cell::StaticCell;
    use rusty_gb::output::spi::DisplayParts;
    use hal::fugit::RateExtU32;
    use embedded_hal::digital::{ErrorType, OutputPin};
    use hal::gpio::FunctionSpi;

    static DISPLAY_BUF: StaticCell<[u8; 512]> = StaticCell::new();


    #[global_allocator]
    static ALLOCATOR: Heap = Heap::empty();

    /// What each bootrom looks for at the start of flash.
    ///
    /// The RP2350 wants a signed image definition in `.start_block`, which it
    /// reads in place. The RP2040 has no such block: its bootrom copies the
    /// first 256 bytes of flash into SRAM and executes them, and that stub is
    /// what configures the QSPI interface for execute-in-place -- so on that
    /// part nothing else in flash is reachable until it has run. Both are
    /// placed by the board's script in `memory/`.
    #[cfg(rp2350)]
    #[link_section = ".start_block"]
    #[used]
    pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

    #[cfg(rp2040)]
    #[link_section = ".boot2"]
    #[used]
    pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

    /// Both boards carry the same 12 MHz crystal.
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

        // The timer block is deliberately not claimed here: `platform::micros`
        // reads its free-running counter without taking ownership, so the
        // emulator core needs no HAL handle threaded through it.
        //
        // `clocks::init` stands in for `hal::clocks::init_clocks_and_plls`,
        // which brings each part up at its nominal speed. This is the same
        // sequence with the core voltage -- and, on the RP2350, the flash
        // divisor -- moved first, so that clk_sys can land at
        // `clocks::SYS_MHZ` instead. The peripherals that owns differ per part,
        // which is the whole of why these two calls are not one.
        #[cfg(rp2350)]
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

        #[cfg(rp2040)]
        let clocks = crate::clocks::init(
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.VREG_AND_CHIP_RESET,
            &mut pac.RESETS,
            &mut watchdog,
        )
        .unwrap();

        #[cfg(rp2350)]
        let timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);
        #[cfg(rp2040)]
        let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

        // Claimed before USB, and that order is load-bearing on the RP2040.
        //
        // `Pins::new` is what brings IO_BANK0 and PADS_BANK0 out of reset, and
        // the RP2040-E5 workaround inside `UsbBus::new` asserts that they
        // already are: it drives DP through GPIO15's pad to hold a line-state J
        // that the USB PHY cannot be made to hold on its own, so a bank still
        // in reset would leave it silently doing nothing. The assert fires
        // first instead -- and under `panic-halt` that is an indistinguishable
        // hang, with no USB to report it over and no core 1 to light the
        // screen. Nothing here needs USB, so the cheap fix is to go first.
        let sio = hal::Sio::new(pac.SIO);
        let pins = hal::gpio::Pins::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, &mut pac.RESETS);

        // Diagnostics go out over USB CDC rather than defmt/RTT, so that
        // reading them needs no debug probe. From here the stack services
        // itself off USBCTRL_IRQ.
        #[cfg(rp2350)]
        crate::usb_serial::init(pac.USB, pac.USB_DPRAM, clocks.usb_clock, &mut pac.RESETS);
        #[cfg(rp2040)]
        crate::usb_serial::init(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            &mut pac.RESETS,
        );
        crate::usb_serial::wait_for_host();

        // Read back rather than printed from a constant: this is the frequency
        // the PLL actually locked to, so an overclock that silently fell back
        // is visible in the first line out of the port rather than only as
        // disappointing FPS. The board is named alongside it because both
        // builds enumerate under the same VID/PID, so this line is the only
        // thing that says which image is actually on the board in front of you.
        use core::fmt::Write as _;
        use hal::Clock as _;
        let board = if cfg!(rp2040) { "Pico 1 (RP2040)" } else { "Pico 2 (RP2350)" };
        let _ = writeln!(
            crate::usb_serial::Serial,
            "rusty-gb on {}: clk_sys {} MHz",
            board,
            clocks.system_clock.freq().to_MHz()
                );

        let sclk = pins.gpio2.into_function::<FunctionSpi>();
        let mosi = pins.gpio3.into_function::<FunctionSpi>();
        let cs = pins.gpio5.into_push_pull_output();
        let dc = pins.gpio6.into_push_pull_output();
        let rst = pins.gpio7.into_push_pull_output();

        let mut backlight = pins.gpio8.into_push_pull_output();
        backlight.set_high().unwrap();


        let spi = hal::Spi::<_, _, _, 8>::new(
            pac.SPI0,
            (
                mosi,
                sclk,
            ),
        ).init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            4.MHz(),
            embedded_hal::spi::MODE_3,
        );


        let output = output::spi::SPI::new(
            &mut pac.PSM,
            &mut pac.PPB,
            sio.fifo,
            DisplayParts {
                spi,
                cs: cs,
                dc: dc,
                rst: rst,
                led: backlight,
                timer,
            },
        );
        let input = input::Dummy::new();

        // Both sinks are the one CDC port -- `Serial` is a unit struct, so these
        // are two handles to the same device, kept separate only so the Game
        // Boy's serial stream is not interleaved mid-token with diagnostics.
        let mut emu = Emulator::new(Rom::mapped(ROM_IMAGE), input, output);
        emu.run(
            60 * 1000,
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
