use crate::hal;
use crate::output::{Output, SCREEN_WIDTH};
use alloc::string::String;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{interface::SpiInterface, models::ILI9225Rgb565, Builder};
use static_cell::StaticCell;
use core::sync::atomic::{AtomicU32, Ordering};
use hal::gpio::{bank0, FunctionSio, FunctionSpi, Pin, PullDown, SioOutput};
use hal::multicore::{Multicore, Stack};
use hal::pac;
use hal::sio::{Sio, SioFifo};
use core::sync::atomic::{AtomicBool};
use embedded_hal::digital::{ErrorType, OutputPin};
use mipidsi::{
    options::{ColorInversion, ColorOrder, Orientation, Rotation},
};

/// The 1 MHz timer, which this module wants only as `mipidsi`'s reset and
/// init delay.
///
/// The RP2350 has two of them and its HAL makes which one a type parameter;
/// the RP2040 has one and does not.
#[cfg(rp2350)]
pub type DelayTimer = hal::Timer<hal::timer::CopyableTimer0>;
#[cfg(rp2040)]
pub type DelayTimer = hal::Timer;

static CS_STICKY: AtomicBool = AtomicBool::new(false);

type Out<I> = Pin<I, FunctionSio<SioOutput>, PullDown>;
pub static CORE1_STATUS: AtomicU32 = AtomicU32::new(0);

static CORE1_STACK: Stack<8192> = Stack::new();

// mipidsi borrows this for the interface's whole lifetime.
static DISPLAY_BUF: StaticCell<[u8; 512]> = StaticCell::new();

pub type SpiBus = hal::Spi<
    hal::spi::Enabled,
    pac::SPI0,
    (
        Pin<bank0::Gpio3, FunctionSpi, PullDown>,
        Pin<bank0::Gpio2, FunctionSpi, PullDown>,
    ),
    8,
>;

pub struct DisplayParts {
    pub spi: SpiBus,
    pub cs: Out<bank0::Gpio5>,
    pub dc: Out<bank0::Gpio6>,
    pub rst: Out<bank0::Gpio7>,
    pub led: Out<bank0::Gpio8>,
    pub timer: DelayTimer,
}



pub struct SPI {
    fifo: SioFifo,
    frames: u32,
    dropped: u32,
}

impl SPI {
    pub fn new(
        psm: &mut pac::PSM,
        ppb: &mut pac::PPB,
        mut fifo: SioFifo,
        parts: DisplayParts,
    ) -> Self {
        let stack = CORE1_STACK.take().unwrap();

        // Scoped so the borrow of `fifo` ends before it is moved into `Self`.
        {
            let mut mc = Multicore::new(psm, ppb, &mut fifo);
            mc.cores()[1].spawn(stack, move || core1(parts)).unwrap();
        }

        Self { fifo, frames: 0, dropped: 0 }
    }

    pub fn dropped(&self) -> u32 {
        self.dropped
    }
}


fn core1(parts: DisplayParts) -> ! {
    CORE1_STATUS.store(1, Ordering::Relaxed);
    let DisplayParts { spi, mut cs, dc, rst, led, mut timer } = parts;
    let mut sio = Sio::new(unsafe { pac::Peripherals::steal().SIO });
    CORE1_STATUS.store(2, Ordering::Relaxed);

    // Failures here are silent under `panic-halt`.
    let spi_dev = ExclusiveDevice::new(spi, cs, timer).unwrap();
    CORE1_STATUS.store(3, Ordering::Relaxed);

    let di = SpiInterface::new(spi_dev, dc, DISPLAY_BUF.init([0u8; 512]));
    let mut display = Builder::new(ILI9225Rgb565, di)
        .reset_pin(rst)
        .display_offset(0, 0)
        .display_size(176, 220)
        .color_order(ColorOrder::Rgb)
        .invert_colors(ColorInversion::Normal)
        .orientation(Orientation::new().rotate(Rotation::Deg0))
        .init(&mut timer)
        .unwrap();
        CS_STICKY.store(true, Ordering::Relaxed); // CS stays low from here
    CORE1_STATUS.store(4, Ordering::Relaxed);

    display.clear(Rgb565::RED).unwrap();
    CORE1_STATUS.store(5, Ordering::Relaxed);

    let mut seen = 0;
    loop {
        let _frame = sio.fifo.read_blocking();
        let c = if seen % 2 == 0 { Rgb565::RED } else { Rgb565::BLUE };
        match display.clear(c) {
            Ok(()) => CORE1_STATUS.store(6, Ordering::Relaxed),
            Err(_) => CORE1_STATUS.store(99, Ordering::Relaxed),
        }
        seen += 1;
    }
}

impl Output for SPI {
    fn write_line(&mut self, _y: u16, _line: &[u8; SCREEN_WIDTH]) {
        // Fill the back buffer here.
    }

    fn refresh(&mut self) -> bool {
        self.frames = self.frames.wrapping_add(1);

        // Non-blocking: a full FIFO means core1 is behind, and dropping the
        // frame beats stalling emulation on the display.
        if self.fifo.is_write_ready() {
            self.fifo.write(self.frames);
        } else {
            self.dropped = self.dropped.wrapping_add(1);
        }

        true
    }

    fn set_diagnostics(&mut self, _diagnostics: String) {}
}