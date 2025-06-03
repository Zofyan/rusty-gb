#![no_std]
#![no_main]
extern crate alloc;

use panic_halt as _;
use alloc::boxed::Box;
use alloc::string::String;
use core::fmt::Debug;
use core::ops::Deref;
use defmt::println;
use embedded_alloc::Heap;
use crate::emulator::Emulator;
use usbd_serial::SerialPort;
use usb_device::{class_prelude::*, prelude::*};
//use peak_alloc::PeakAlloc;
use rp235x_hal as hal;

#[global_allocator]
static ALLOCATOR: Heap = Heap::empty();

//#[global_allocator]
//static PEAK_ALLOC: PeakAlloc = PeakAlloc;

mod cpu;
mod bus;
mod emulator;
mod register;
mod memory;
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
#[hal::entry]
fn main() -> ! {
    let output: Box<dyn output::Output> = match "Dummy" {
        //"Terminal" => Box::new(output::terminal::Terminal::new(4f64)),
        "Dummy" => Box::new(output::dummy::Dummy::new()),
        //"LCD" => Box::new(output::lcd::LCD::new(4)),
        _ => panic!("Unknown output type"),
    };
    let input = input::Dummy::new();

    let mut pac = hal::pac::Peripherals::take().unwrap();// Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
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
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USB,
        pac.USB_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    // Set up the USB Communications Class Device driver
    let mut serial = SerialPort::new(&usb_bus);

    let mut emu = Emulator::new(
        "Pokemon Red.gb",
        input,
        output,
        timer
    );

    emu.run(60*10)
    //let peak_mem = PEAK_ALLOC.peak_usage_as_kb();
    //println!("The max amount that was used {}", peak_mem);
}
