//! USB CDC serial, used as the Pico's diagnostic sink.
//!
//! The alternative on this board is defmt over RTT, but draining an RTT buffer
//! needs a SWD debug probe. CDC needs only the USB cable that already powers
//! the board, so this is what [`rusty_gb::emulator::Emulator::run`] writes its FPS
//! line and the Game Boy's serial port into.
//!
//! # Why this runs off an interrupt
//!
//! USB enumeration requires the device to answer control transfers within a few
//! milliseconds of being asked. `Emulator::run` is a tight loop that only
//! reaches the end of a frame every 16 ms or worse, so polling the stack from
//! there would make enumeration unreliable at best. Servicing `USBCTRL_IRQ`
//! instead decouples USB timing from the emulator entirely: the frame loop
//! never has to care, and the port stays alive even after `run` returns.
//!
//! # Rebooting into BOOTSEL from the host
//!
//! The same interrupt watches for the Arduino-style 1200 baud touch (see
//! [`BOOTSEL_TOUCH_BAUD`]), which is what lets a host flash the board without
//! anyone reaching for the BOOTSEL button. `tools/flash.ps1` drives it.

use alloc::boxed::Box;
use core::cell::RefCell;
use core::fmt;
use critical_section::Mutex;
use rp235x_hal as hal;
use hal::pac::interrupt;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid};
use usb_device::UsbError;
use usbd_serial::SerialPort;

type Bus = hal::usb::UsbBus;

/// Raspberry Pi's vendor ID with the SDK's CDC-UART product ID. Reusing the
/// pair the Pico SDK ships means hosts already have a driver bound to it, so
/// the port appears without any per-machine setup.
const VID_PID: UsbVidPid = UsbVidPid(0x2e8a, 0x000a);

/// How long [`wait_for_host`] gives a terminal to attach.
const ATTACH_TIMEOUT_US: u64 = 10_000_000;

/// Opening the port at this baud and then dropping DTR reboots the board into
/// BOOTSEL.
///
/// The convention comes from Arduino and every RP2040/RP2350 toolchain that
/// followed it, which is the point: it needs no client beyond something that
/// can open a serial port, so `picotool` can flash a board that is face down in
/// a case. Nothing else here reads the line coding -- CDC over USB has no wire
/// to be slow -- so the value is free to be a sentinel, and 1200 baud is the
/// one hosts already know to use.
const BOOTSEL_TOUCH_BAUD: u32 = 1200;

struct Usb {
    dev: UsbDevice<'static, Bus>,
    port: SerialPort<'static, Bus>,
}

impl Usb {
    fn poll(&mut self) -> bool {
        self.dev.poll(&mut [&mut self.port])
    }

    /// A host is present and has opened the port.
    ///
    /// `dtr` is what distinguishes "enumerated by the OS" from "someone is
    /// actually listening"; `screen`, `picocom` and `minicom` all assert it on
    /// open.
    fn ready(&self) -> bool {
        self.dev.state() == UsbDeviceState::Configured && self.port.dtr()
    }

    /// The host has asked for BOOTSEL by opening at [`BOOTSEL_TOUCH_BAUD`] and
    /// closing again.
    ///
    /// Both halves are required. DTR alone is every terminal attaching, and the
    /// baud alone is a host that has set the line coding but not yet let go, so
    /// rebooting on either one on its own would fire while a flashing tool
    /// still had the port open.
    fn bootsel_requested(&self) -> bool {
        self.port.line_coding().data_rate() == BOOTSEL_TOUCH_BAUD && !self.port.dtr()
    }
}

/// The stack is reached from both thread mode (writes) and the USB interrupt,
/// so it lives behind a critical section rather than being owned by `main`.
static USB: Mutex<RefCell<Option<Usb>>> = Mutex::new(RefCell::new(None));

/// Brings up the device and hands the stack over to `USBCTRL_IRQ`.
///
/// Must run after the global allocator is initialised, and the caller keeps no
/// handle: everything afterwards goes through [`Serial`].
pub fn init(
    usb: hal::pac::USB,
    dpram: hal::pac::USB_DPRAM,
    clock: hal::clocks::UsbClock,
    resets: &mut hal::pac::RESETS,
) {
    let bus = Bus::new(usb, dpram, clock, true, resets);

    // The device and the port both borrow the allocator for as long as they
    // live, which is forever, so it has to be `&'static`. Leaking one boxed
    // allocator at boot is the cheapest way to say that; it is a few hundred
    // bytes and happens once, so the ERAM heap does not miss it.
    let alloc: &'static UsbBusAllocator<Bus> = Box::leak(Box::new(UsbBusAllocator::new(bus)));

    // The port must be constructed before the device: `UsbDeviceBuilder::build`
    // freezes the interface and endpoint allocation.
    let port = SerialPort::new(alloc);
    let dev = UsbDeviceBuilder::new(alloc, VID_PID)
        .strings(&[StringDescriptors::default()
            .manufacturer("rusty-gb")
            .product("rusty-gb")
            .serial_number("dmg")])
        .expect("usb string descriptors")
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    critical_section::with(|cs| {
        USB.borrow(cs).replace(Some(Usb { dev, port }));
    });

    // SAFETY: the stack is in place above, so the handler has something to
    // service the moment this unmasks.
    unsafe { cortex_m::peripheral::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ) };
}

/// Blocks until a terminal opens the port, or [`ATTACH_TIMEOUT_US`] elapses.
///
/// Without this the emulator would race USB enumeration and the first second of
/// output would vanish into an unopened port. The timeout is what keeps the
/// board useful standalone: on a bare USB charger nothing ever attaches, and it
/// should still boot and run rather than hang.
///
/// Returns whether a host actually turned up.
pub fn wait_for_host() -> bool {
    let start = rusty_gb::platform::micros();
    loop {
        if critical_section::with(|cs| {
            USB.borrow(cs).borrow().as_ref().is_some_and(Usb::ready)
        }) {
            return true;
        }
        if rusty_gb::platform::micros() - start > ATTACH_TIMEOUT_US {
            return false;
        }
    }
}

/// [`fmt::Write`] sink over the CDC port.
///
/// Writes are lossy on purpose. This carries diagnostics, so a host that stops
/// draining the port -- or was never there -- must not be able to stall the
/// emulator; bytes are dropped instead.
pub struct Serial;

/// Bound on retries per `write_str`, so a wedged host costs a fixed number of
/// polls rather than the rest of the run.
const MAX_ATTEMPTS: usize = 64;

impl fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut buf = s.as_bytes();
        let mut attempts = 0;

        while !buf.is_empty() && attempts < MAX_ATTEMPTS {
            attempts += 1;

            let written = critical_section::with(|cs| {
                let mut guard = USB.borrow(cs).borrow_mut();
                let usb = guard.as_mut()?;
                if usb.dev.state() != UsbDeviceState::Configured {
                    return None;
                }
                match usb.port.write(buf) {
                    Ok(n) => Some(n),
                    // The IRQ is masked in here, so nothing else can move the
                    // endpoint along -- poll it directly and retry.
                    Err(UsbError::WouldBlock) => {
                        usb.poll();
                        Some(0)
                    }
                    Err(_) => None,
                }
            });

            match written {
                Some(n) => buf = &buf[n..],
                // Not attached, or a fatal endpoint error: drop the rest.
                None => break,
            }
        }

        Ok(())
    }
}

#[interrupt]
fn USBCTRL_IRQ() {
    let reboot = critical_section::with(|cs| {
        let mut guard = USB.borrow(cs).borrow_mut();
        let Some(usb) = guard.as_mut() else {
            return false;
        };
        usb.poll();
        usb.bootsel_requested()
    });

    // Outside the critical section and outside the borrow: `reboot` never
    // returns, so anything still held here would be held forever -- and the ROM
    // call it makes has no business running with interrupts masked.
    if reboot {
        hal::reboot::reboot(
            // Leave both interfaces up. Disabling either only narrows what the
            // host can do with the board it just rebooted, and the mass storage
            // one is the fallback when picotool cannot see it.
            hal::reboot::RebootKind::BootSel {
                picoboot_disabled: false,
                msd_disabled: false,
            },
            hal::reboot::RebootArch::Normal,
        );
    }
}
