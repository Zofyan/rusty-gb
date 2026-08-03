use crate::hal;
use crate::output::{Output, SCREEN_WIDTH, SCREEN_HEIGHT};
use crate::output::panel::ili9225::Ili9225;
use alloc::string::String;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{interface::SpiInterface, models::ILI9225Rgb565, Builder};
use static_cell::StaticCell;
use core::cell::UnsafeCell;
use core::sync::atomic::{fence, AtomicU32, Ordering};
use hal::gpio::{bank0, FunctionSio, FunctionSpi, Pin, PullDown, SioOutput};
use hal::multicore::{Multicore, Stack};
use hal::pac;
use hal::sio::{Sio, SioFifo};
use core::sync::atomic::{AtomicBool};
use embedded_hal::digital::{ErrorType, OutputPin};
use mipidsi::{
    options::{ColorInversion, ColorOrder, Orientation, Rotation},
};
#[cfg(rp2350)]
pub type DelayTimer = hal::Timer<hal::timer::CopyableTimer0>;
#[cfg(rp2040)]
pub type DelayTimer = hal::Timer;
static CS_STICKY: AtomicBool = AtomicBool::new(false);

type Out<I> = Pin<I, FunctionSio<SioOutput>, PullDown>;
pub static CORE1_STATUS: AtomicU32 = AtomicU32::new(0);

static CORE1_STACK: Stack<8192> = Stack::new();

// mipidsi borrows this for the interface's whole lifetime. Each chunk is one
// SPI transaction (CS toggle + flush), so bigger is meaningfully faster.
static DISPLAY_BUF: StaticCell<[u8; 4096]> = StaticCell::new();

// Panel geometry, and where the emulator's screen lands on it.
const PANEL_W: u16 = 176;
const PANEL_H: u16 = 220;
const DST_X: u16 = (PANEL_W - SCREEN_WIDTH as u16) / 2;
const DST_Y: u16 = (PANEL_H - SCREEN_HEIGHT as u16) / 2;

// ---------------------------------------------------------------------------
// Shared framebuffers.
//
// Two byte-per-pixel buffers holding raw palette indices; expansion to RGB565
// happens on core1 during the blit. Exactly one core owns each buffer at any
// moment, and ownership moves by passing the buffer index through the SIO
// FIFO. A core must never touch a buffer whose index it has handed over.
// ---------------------------------------------------------------------------
const FB_LEN: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

struct FrameBuffers(UnsafeCell<[[u8; FB_LEN]; 2]>);

// Safety: upheld by the token protocol described above.
unsafe impl Sync for FrameBuffers {}

impl FrameBuffers {
    /// Safety: the caller must currently hold the token for `idx`.
    #[allow(clippy::mut_from_ref)]
    unsafe fn get(&self, idx: usize) -> &mut [u8; FB_LEN] {
        &mut (*self.0.get())[idx]
    }
}

static FRAMES: FrameBuffers = FrameBuffers(UnsafeCell::new([[0u8; FB_LEN]; 2]));

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
    /// Buffer core0 is currently filling.
    buf: usize,
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

        Self { fifo, buf: 0, frames: 0, dropped: 0 }
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

    let di = SpiInterface::new(spi_dev, dc, DISPLAY_BUF.init([0u8; 4096]));
    let mut display = Builder::new(Ili9225, di)
        .reset_pin(rst)
        .display_offset(0, 0)
        .display_size(176, 220)
        .color_order(ColorOrder::Rgb)
        .invert_colors(ColorInversion::Inverted)
        .orientation(Orientation::new().rotate(Rotation::Deg180))
        .init(&mut timer)
        .unwrap();
        CS_STICKY.store(true, Ordering::Relaxed); // CS stays low from here
    CORE1_STATUS.store(4, Ordering::Relaxed);
    display
        .set_pixels(0, 0, 175, 219, core::iter::repeat(Rgb565::CSS_FUCHSIA).take(176 * 220))
        .unwrap();
    CORE1_STATUS.store(5, Ordering::Relaxed);

    // Green gets 6 bits, red and blue 5, so a neutral grey needs roughly
    // double the green value.
    let palette: [Rgb565; 4] = [
        Rgb565::new(27, 54, 27),
        Rgb565::new(18, 36, 18),
        Rgb565::new(8, 16, 8),
        Rgb565::new(0, 0, 0),
    ];

    // Buffer 1 is free; core0 starts out filling buffer 0.
    fence(Ordering::Release);
    sio.fifo.write_blocking(1);

    loop {
        let idx = (sio.fifo.read_blocking() & 1) as usize;
        fence(Ordering::Acquire);

        // Safety: core0 handed us this buffer and will not touch it until we
        // hand it back below.
        let fb = unsafe { FRAMES.get(idx) };

        // set_pixels goes through Model::write_memory_start (0x22). Do not use
        // clear()/fill_solid here: mipidsi hardcodes DCS 0x2C on that path,
        // which the ILI9225 ignores.
        match display.set_pixels(
            DST_X,
            DST_Y,
            DST_X + SCREEN_WIDTH as u16 - 1,
            DST_Y + SCREEN_HEIGHT as u16 - 1,
            fb.iter().map(|&v| palette[(v & 3) as usize]),
        ) {
            Ok(()) => CORE1_STATUS.store(6, Ordering::Relaxed),
            Err(_) => CORE1_STATUS.store(99, Ordering::Relaxed),
        }

        fence(Ordering::Release);
        sio.fifo.write_blocking(idx as u32);
    }
}

impl Output for SPI {
    fn write_line(&mut self, y: u16, line: &[u8; SCREEN_WIDTH]) {
        let y = y as usize;
        if y >= SCREEN_HEIGHT {
            return;
        }

        // Safety: core0 owns `self.buf` until refresh() hands it over.
        let fb = unsafe { FRAMES.get(self.buf) };
        fb[y * SCREEN_WIDTH..(y + 1) * SCREEN_WIDTH].copy_from_slice(line);
    }

    fn refresh(&mut self) -> bool {
        self.frames = self.frames.wrapping_add(1);

        // core1 returns the index of a buffer it has finished blitting. No
        // token means it is still busy, so keep filling the current buffer and
        // count the frame as dropped rather than stalling emulation.
        match self.fifo.read() {
            Some(free) => {
                fence(Ordering::Acquire);
                let filled = self.buf as u32;
                self.buf = (free & 1) as usize;
                fence(Ordering::Release);
                self.fifo.write(filled);
            }
            None => {
                self.dropped = self.dropped.wrapping_add(1);
            }
        }

        true
    }

    fn set_diagnostics(&mut self, _diagnostics: String) {}
}