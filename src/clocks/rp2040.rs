//! Pico 1 / W / WH (RP2040) clock bring-up, at twice the part's nominal system
//! clock.
//!
//! [`hal::clocks::init_clocks_and_plls`] is the one-call version of everything
//! below and brings the part up at 125 MHz. The Cortex-M0+ is a much smaller
//! core than the Pico 2's M33 -- 16-bit Thumb only, no DSP, no FPU, and a
//! multiply that is not always single-cycle -- so this board is further from
//! full speed than the Pico 2 is, and the clock is the cheapest ground to make
//! up. [`SYS_MHZ`] is the one knob; 125 puts everything back to stock.
//!
//! 250 MHz is out of spec: Raspberry Pi specify 133 MHz. It is the most
//! well-trodden RP2040 overclock there is -- an exact doubling, and a clean
//! 1500 MHz VCO -- but "well-trodden" is not "guaranteed". If the board hangs
//! or the picture breaks up, try 1.20 V ([`VSEL_A::VOLTAGE1_20`]) before
//! suspecting anything else, then drop `SYS_MHZ`.
//!
//! # Why there is no flash divisor step here
//!
//! Its Pico 2 counterpart has one, because on the RP2350 the QSPI divisor has
//! to be scaled by hand to keep flash SCK where the bootrom left it. Neither
//! half of that reasoning carries over:
//!
//! * Flash SCK is `clk_sys / SCKDV`, and the second-stage bootloader in
//!   `.boot2` leaves `SCKDV` at 4. At 250 MHz that is 62.5 MHz, well inside the
//!   133 MHz the Pico's W25Q16JV is rated for on the quad-I/O read `boot2`
//!   uses. Nothing needs correcting -- and unlike the Pico 2, where the divisor
//!   is scaled up and flash bandwidth therefore stands still, here it doubles
//!   along with the core.
//! * Correcting it would be expensive anyway. The RP2040's SSI only accepts
//!   writes to its baud and timing registers while it is disabled, and
//!   disabling it un-maps XIP -- so, unlike the RP2350's QMI, any code that
//!   touches flash timing has to be resident in SRAM before it starts.
//!
//! What does *not* scale is `RXDELAY`, the sample point `boot2` picked, which
//! is counted in system clock cycles and so is half the wall-clock delay it was
//! at 125 MHz. That, rather than SCK, is what puts a ceiling on `SYS_MHZ`.

use rp2040_hal as hal;

use hal::clocks::{ClocksManager, InitError};
use hal::fugit::{HertzU32, RateExtU32};
use hal::pac;
use hal::pac::vreg_and_chip_reset::vreg::VSEL_A;
use hal::pll::{common_configs::PLL_USB_48MHZ, setup_pll_blocking, PLLConfig};
use hal::vreg::set_voltage;
use hal::xosc::setup_xosc_blocking;

/// Target system clock, and the only thing in this file meant to be edited.
pub const SYS_MHZ: u32 = 250;

/// What [`hal::clocks::init_clocks_and_plls`] would have configured.
const STOCK_SYS_MHZ: u32 = 125;

/// `SYS_MHZ * 6` MHz VCO / 6 / 1 -- 1500 MHz at the default, which is the same
/// VCO the stock 125 MHz config runs at, just with a shorter post-divide.
///
/// The fixed post-dividers put a range on [`SYS_MHZ`]: the VCO has to land
/// between 750 and 1600 MHz, so 125 to 266 MHz. Outside that the PLL will not
/// lock and `init` returns `InitError::PllError` rather than running fast.
///
/// The USB PLL keeps its stock 48 MHz, so USB CDC is unaffected by any of this
/// -- only `clk_sys` and `clk_peri` move.
const PLL_SYS_CONFIG: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(SYS_MHZ * 6),
    refdiv: 1,
    post_div1: 6,
    post_div2: 1,
};

/// One 50 mV step above the 1.10 V default.
///
/// This is well under the regulator's 1.30 V limit, so `VREG_CTRL`'s
/// over-voltage protection is left alone.
const OVERCLOCK_VSEL: VSEL_A = VSEL_A::VOLTAGE1_15;

/// Brings up XOSC, both PLLs and the clock tree with `clk_sys` at [`SYS_MHZ`].
///
/// Mirrors [`hal::clocks::init_clocks_and_plls`]'s signature, plus the one
/// peripheral it never touches: `VREG_AND_CHIP_RESET` owns the core regulator,
/// and going faster than stock means it has to move first.
pub fn init(
    xosc_dev: pac::XOSC,
    clocks_dev: pac::CLOCKS,
    pll_sys_dev: pac::PLL_SYS,
    pll_usb_dev: pac::PLL_USB,
    vreg: &mut pac::VREG_AND_CHIP_RESET,
    resets: &mut pac::RESETS,
    watchdog: &mut hal::Watchdog,
) -> Result<ClocksManager, InitError> {
    let xtal_freq_hz = crate::pico::XTAL_FREQ_HZ;

    // Voltage first: the core has to be able to hold the new frequency before
    // anything is running at it.
    raise_core_voltage(vreg);

    let xosc = setup_xosc_blocking(xosc_dev, xtal_freq_hz.Hz()).map_err(InitError::XoscErr)?;

    // A tick every microsecond, from clk_ref. This is what drives TIMER, and
    // therefore `platform::micros` and every FPS figure: it is derived from the
    // crystal rather than from clk_sys, so the numbers below stay honest at any
    // system clock.
    watchdog.enable_tick_generation((xtal_freq_hz / 1_000_000) as u8);

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
/// clock and is not fine here: at [`SYS_MHZ`] it hands the SPI block a 250 MHz
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

/// Steps the core regulator up to [`OVERCLOCK_VSEL`] and waits for it to settle.
///
/// A no-op at the stock clock: 1.10 V is what the datasheet speed is specified
/// at, and paying for extra voltage -- and the heat that comes with it -- is
/// only worth it when [`SYS_MHZ`] is asking for margin the default does not
/// have.
fn raise_core_voltage(vreg: &mut pac::VREG_AND_CHIP_RESET) {
    if SYS_MHZ <= STOCK_SYS_MHZ {
        return;
    }

    set_voltage(vreg, OVERCLOCK_VSEL);

    // The write returning is not the rail having physically got there. Cycles
    // rather than microseconds because this runs before the PLL and before the
    // tick generator, so neither the system clock nor TIMER is at a known rate
    // yet -- but the bound holds either way: 100k cycles is under a millisecond
    // even at the fastest the core could possibly be running here, and rather
    // more than that at the ROSC speed it is actually booting at.
    cortex_m::asm::delay(100_000);
}
