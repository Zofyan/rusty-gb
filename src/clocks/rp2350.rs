//! Pico 2 (RP2350) clock bring-up, at twice the part's nominal system clock.
//!
//! [`hal::clocks::init_clocks_and_plls`] is the one-call version of everything
//! below and brings the part up at 150 MHz, its datasheet maximum. The emulator
//! is CPU-bound at that speed -- roughly 30 fps -- so the clock is where the
//! next doubling comes from, and the whole reason this module exists rather
//! than a one-line change to the PLL config is the ordering around it:
//!
//! * the core voltage has to rise *before* the frequency does, or the first
//!   clock edge at the new speed lands on a core still at 1.10 V, and
//! * the QSPI divisor has to rise *with* it, because both the code and the
//!   cartridge execute in place from flash and SCK is `clk_sys / CLKDIV`.
//!
//! 300 MHz is out of spec. Raspberry Pi specify 150 MHz at 1.10 V; the part is
//! routinely run at double that with a little more core voltage, but "routinely
//! works" is not "guaranteed", and a chip that is unstable here will show it as
//! a hang or a corrupted frame rather than anything graceful. [`SYS_MHZ`] is
//! the one knob: 150 puts everything back to stock, and the PLL config and the
//! flash divisor both follow it.

use rp235x_hal as hal;

use hal::clocks::{ClocksManager, InitError};
use hal::fugit::{HertzU32, RateExtU32};
use hal::pac;
use hal::pll::{common_configs::PLL_USB_48MHZ, setup_pll_blocking, PLLConfig};
use hal::xosc::setup_xosc_blocking;

/// Target system clock. Also the numerator of the flash divisor scaling below,
/// so the two cannot drift apart.
pub const SYS_MHZ: u32 = 300;

/// What [`hal::clocks::init_clocks_and_plls`] would have configured, and the
/// speed the bootrom's flash timing was chosen for.
const STOCK_SYS_MHZ: u32 = 150;

/// 1500 MHz VCO / 5 / 1. The USB PLL keeps its stock 48 MHz, so USB CDC is
/// unaffected by any of this -- only `clk_sys` and `clk_peri` move.
const PLL_SYS_CONFIG: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(SYS_MHZ * 5),
    refdiv: 1,
    post_div1: 5,
    post_div2: 1,
};

/// `VSEL` for 1.15 V, one 50 mV step above the 1.10 V default.
///
/// The regulator's own limit is 1.30 V and this stays well under it, so
/// `disable_voltage_limit` in `VREG_CTRL` is left alone -- the chip keeps its
/// over-voltage protection. 1.15 V is the smallest step that gives 300 MHz
/// margin; a part that will not hold 300 MHz here would want 1.20 V
/// (`0b01101`) before anything else is suspected.
const VSEL_1_15V: u32 = 0b0_1100;

/// Every POWMAN register write needs this in its top half or it is discarded
/// and flagged in `POWMAN.BADPASSWD`.
const POWMAN_PASSWORD: u32 = 0x5AFE_0000;

/// `VREG_CTRL.UNLOCK`. The regulator ignores `VSEL` writes until it is set, and
/// once set it stays set until the next power-on.
const VREG_CTRL_UNLOCK: u32 = 1 << 13;

/// `XIP_NOCACHE_NOALLOC_BASE`: the same flash as `XIP_BASE` (0x1000_0000) seen
/// through a window that neither hits the cache nor fills it.
///
/// This is what the dummy read below has to go through. A read of the ordinary
/// window would be answered by the 16 KiB XIP cache -- likely, since the code
/// doing the reading lives in flash -- and never reach the QMI at all, which is
/// precisely the thing it is there to provoke.
const XIP_NOCACHE_BASE: u32 = 0x1400_0000;

/// Brings up XOSC, both PLLs and the clock tree with `clk_sys` at [`SYS_MHZ`].
///
/// Mirrors [`hal::clocks::init_clocks_and_plls`]'s signature, plus the two
/// peripherals it never touches: POWMAN owns the core regulator and QMI owns
/// flash timing, and going faster than stock means both have to move first.
pub fn init(
    xosc_dev: pac::XOSC,
    clocks_dev: pac::CLOCKS,
    pll_sys_dev: pac::PLL_SYS,
    pll_usb_dev: pac::PLL_USB,
    powman: pac::POWMAN,
    qmi: pac::QMI,
    resets: &mut pac::RESETS,
    watchdog: &mut hal::Watchdog,
) -> Result<ClocksManager, InitError> {
    let xtal_freq_hz = crate::pico::XTAL_FREQ_HZ;

    // Voltage first: the core has to be able to hold the new frequency before
    // anything is running at it.
    raise_core_voltage(&powman);

    let xosc = setup_xosc_blocking(xosc_dev, xtal_freq_hz.Hz()).map_err(InitError::XoscErr)?;

    // A tick every microsecond, from clk_ref. This is what drives TIMER0, and
    // therefore `platform::micros` and every FPS figure: it is derived from the
    // crystal rather than from clk_sys, so the numbers below stay honest at any
    // system clock.
    watchdog.enable_tick_generation((xtal_freq_hz / 1_000_000) as u16);

    // Then flash, while clk_sys is still slow.
    slow_flash_divisor(&qmi);

    let mut clocks = ClocksManager::new(clocks_dev);

    let pll_sys = setup_pll_blocking(
        pll_sys_dev,
        xosc.operating_frequency(),
        PLL_SYS_CONFIG,
        &mut clocks,
        resets,
    )
    .map_err(InitError::PllError)?;
    let pll_usb = setup_pll_blocking(
        pll_usb_dev,
        xosc.operating_frequency(),
        PLL_USB_48MHZ,
        &mut clocks,
        resets,
    )
    .map_err(InitError::PllError)?;

    clocks
        .init_default(&xosc, &pll_sys, &pll_usb)
        .map_err(InitError::ClockError)?;

    peripherals_off_usb_pll(&mut clocks, &pll_usb)?;

    Ok(clocks)
}

/// Repoints `clk_peri` at the USB PLL, so peripherals do not get overclocked
/// along with the core.
///
/// `init_default` wires `clk_peri` to `clk_sys`, which is fine at the stock
/// clock and is not fine here: at [`SYS_MHZ`] it hands the SPI block a 300 MHz
/// input, double what the datasheet allows it, and the symptom is a display
/// that never acknowledges its init sequence rather than an error. Nothing on
/// this board wants `clk_peri` to track the core -- the SPI baud rate is
/// requested in Hz and the driver divides down to it either way -- so pointing
/// it at the 48 MHz USB PLL, which is already running and which [`SYS_MHZ`]
/// does not move, decouples every peripheral from the overclock for good.
///
/// 48 MHz is not a constraint worth worrying about: the SSP divides it by at
/// least 2, so it still offers 24 MHz of SPI against the 4 MHz the panel is
/// driven at.
fn peripherals_off_usb_pll(
    clocks: &mut ClocksManager,
    pll_usb: &hal::pll::PhaseLockedLoop<hal::pll::Locked, pac::PLL_USB>,
) -> Result<(), InitError> {
    use hal::clocks::{Clock as _, ClockSource as _};

    clocks
        .peripheral_clock
        .configure_clock(pll_usb, pll_usb.get_freq())
        .map_err(InitError::ClockError)
}

/// Steps the core regulator up to [`VSEL_1_15V`] and waits for it to settle.
///
/// A no-op at the stock clock: 1.10 V is what 150 MHz is specified at, and
/// paying for extra voltage -- and the heat that comes with it -- is only worth
/// it when [`SYS_MHZ`] is asking for margin the default does not have.
fn raise_core_voltage(powman: &pac::POWMAN) {
    if SYS_MHZ <= STOCK_SYS_MHZ {
        return;
    }

    // The unlock bit is sticky, so this is a no-op on the second boot after a
    // warm reset. Read-modify-write rather than a bare write: `VREG_CTRL` also
    // holds the high-temperature trip point, which we have no business
    // clearing.
    let ctrl = powman.vreg_ctrl().read().bits() & 0xFFFF;
    powman
        .vreg_ctrl()
        .write(|w| unsafe { w.bits(POWMAN_PASSWORD | ctrl | VREG_CTRL_UNLOCK) });

    while powman.vreg().read().update_in_progress().bit_is_set() {}
    powman
        .vreg()
        .write(|w| unsafe { w.bits(POWMAN_PASSWORD | (VSEL_1_15V << 4)) });
    while powman.vreg().read().update_in_progress().bit_is_set() {}

    // `update_in_progress` clearing means the request was accepted, not that the
    // rail has physically got there. Cycles rather than microseconds because
    // this runs before the PLL and before the tick generator, so neither the
    // system clock nor TIMER0 is at a known rate yet -- but the bound holds
    // either way: 100k cycles is 0.7 ms even at the fastest the core could
    // possibly be running here, and milliseconds more than that at boot speeds.
    cortex_m::asm::delay(100_000);
}

/// Scales `M0_TIMING.CLKDIV` so flash SCK comes out where it is today.
///
/// SCK is `clk_sys / CLKDIV`, so doubling the system clock without touching
/// this doubles the QSPI bus too -- and the sample point (`RXDELAY`) the
/// bootrom picked was picked for the slower bus, which is the failure mode that
/// looks like random corruption in code and cartridge data alike rather than a
/// clean fault. Scaling the divisor by the same factor leaves the QSPI bus
/// running at exactly the frequency the bootrom configured it for, so flash
/// reads cost the same wall-clock time they do now while everything that hits
/// the XIP cache gets the full speedup.
///
/// The cost is that flash bandwidth does not scale with the core, so the
/// speedup is short of 2x by however much the 16 KiB XIP cache misses. Trading
/// that back -- a smaller divisor plus a larger `RXDELAY` -- is worth trying
/// once the clock itself is known good, but it is a separate experiment with
/// its own way of going wrong.
fn slow_flash_divisor(qmi: &pac::QMI) {
    // M1 is the second chip select, unpopulated on a Pico 2: nothing executes
    // from it, so its timing is left alone.
    let current = qmi.m0_timing().read().clkdiv().bits();

    // 0 encodes 256, already the slowest this divider goes.
    if current == 0 {
        return;
    }

    let scale = SYS_MHZ.div_ceil(STOCK_SYS_MHZ) as u8;
    let scaled = current.saturating_mul(scale);
    if scaled == current {
        return;
    }

    qmi.m0_timing().modify(|_, w| unsafe { w.clkdiv().bits(scaled) });

    // The divisor lands on the QMI's next access, so without a read that
    // actually goes to flash the system clock could rise first and the very
    // next fetch would run at the old divisor against the new clock.
    unsafe { core::ptr::read_volatile(XIP_NOCACHE_BASE as *const u32) };
    cortex_m::asm::dsb();
}
