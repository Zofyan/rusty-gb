#![no_std]
#![no_main]
extern crate alloc;

use core::fmt::Write;
use core::mem::MaybeUninit;
use defmt::println;
use embedded_alloc::Heap;
use panic_halt as _;
use rp235x_hal as hal;

use crate::emulator::Emulator;
use crate::rom::Rom;

#[global_allocator]
static ALLOCATOR: Heap = Heap::empty();

mod cpu;
mod bus;
mod emulator;
mod register;
mod memory;
mod platform;
mod ppu;
mod fetcher;
mod output;
mod window_fetcher;
mod input;
mod mbc;
mod rom;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

pub(crate) const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// The cartridge, linked into XIP flash.
///
/// A plain `static` lands in `.rodata`, which on this target is memory-mapped
/// flash, so [`Rom::mapped`] can address it in place at no cost in SRAM -- which
/// is the whole reason `Rom` holds `&'static [u8]` rather than owning a copy.
static ROM_IMAGE: &[u8] = include_bytes!("../test-roms/Pokemon Red.gb");

/// Backs the ERAM `Vec` in [`memory::Memory`] and nothing else, so it only has
/// to cover the largest cartridge RAM this emulator maps (32 KiB, MBC3).
const HEAP_SIZE: usize = 64 * 1024;
static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

/// The Game Boy's serial port, forwarded to defmt.
///
/// Test ROMs report their results by writing bytes to SB, which is what
/// [`Emulator::run`] drains into this sink.
struct Serial;

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        println!("{=str}", s);
        Ok(())
    }
}

#[hal::entry]
fn main() -> ! {
    // Must precede the first allocation. The previous revision never called
    // this, so the ERAM `Vec` would have faulted on the first cartridge with
    // save RAM.
    unsafe { ALLOCATOR.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }

    let mut pac = hal::pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // TIMER0 is deliberately not claimed here: `platform::micros` reads its
    // free-running counter without taking ownership, so the emulator core needs
    // no HAL handle threaded through it.
    let _clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let output = output::dummy::Dummy::new();
    let input = input::Dummy::new();

    let mut emu = Emulator::new(Rom::mapped(ROM_IMAGE), input, output);
    emu.run(60 * 10, &mut Serial);

    loop {
        cortex_m::asm::wfi();
    }
}
