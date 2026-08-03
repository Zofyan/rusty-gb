//! mipidsi's `ILI9225Rgb565` with its power-on block corrected.
//!
//! The stock model writes five power-control values to registers shifted one
//! position out of step. Its own comments give it away -- each describes a
//! different register than the one being written, and re-pairing each value
//! with the register its comment names yields the canonical ILI9225 sequence.
//! The effect is that GVDD and VCOMH/VCOML never get set, so the panel gets no
//! drive voltage and shows uniform white with the backlight lit.
//!
//! Only `init` and `software_reset` are ours. Everything else delegates to the
//! stock model, so window addressing, sleep/wake, tearing and rotation stay
//! whatever upstream says they are.
//!
//! Usage: replace `ILI9225Rgb565` with `Ili9225Fixed` in `Builder::new`.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal::delay::DelayNs;
use mipidsi::{
    dcs::{InterfaceExt, SetAddressMode},
    interface::Interface,
    models::{Model, ModelInitError, ILI9225Rgb565, SoftResetILI9225},
    options::{ColorInversion, ModelOptions, Rotation, TearingEffect},
};

pub struct Ili9225;

impl Model for Ili9225 {
    type ColorFormat = Rgb565;
    const FRAMEBUFFER_SIZE: (u16, u16) = ILI9225Rgb565::FRAMEBUFFER_SIZE;
    const RESET_DURATION: u32 = ILI9225Rgb565::RESET_DURATION;

    fn init<DELAY, DI>(
        &mut self,
        di: &mut DI,
        delay: &mut DELAY,
        options: &ModelOptions,
    ) -> Result<SetAddressMode, ModelInitError<DI::Error>>
    where
        DELAY: DelayNs,
        DI: Interface,
    {
        // Power everything down first.
        for r in [0x10u8, 0x11, 0x12, 0x13, 0x14] {
            di.write_raw(r, &[0x00, 0x00])?;
        }
        delay.delay_us(40_000);

        // ---- the fix -----------------------------------------------------
        // Each value on the register it actually configures.
        di.write_raw(0x11, &[0x00, 0x18])?; // APON, PON, AON, VCI1EN, VC
        di.write_raw(0x12, &[0x61, 0x21])?; // BT, DC1, DC2, DC3
        di.write_raw(0x13, &[0x00, 0x6F])?; // GVDD
        di.write_raw(0x14, &[0x49, 0x5F])?; // VCOMH / VCOML
        di.write_raw(0x10, &[0x08, 0x00])?; // SAP, DSTB, STB
        delay.delay_us(10_000);
        di.write_raw(0x11, &[0x10, 0x3B])?;
        delay.delay_us(30_000);
        // ------------------------------------------------------------------

        di.write_raw(0x02, &[0x01, 0x00])?; // LCD AC driving, 1-line inversion

        // Writes DRIVER_OUTPUT_CTRL (0x01) and ENTRY_MODE (0x03) from the
        // builder's rotation and colour order. Upstream's logic, unchanged.
        ILI9225Rgb565.update_options(di, options)?;

        di.write_raw(0x07, &[0x00, 0x00])?; // display off while configuring

        for (r, v) in [
            (0x08u8, [0x08u8, 0x08u8]), // blank period
            (0x0B, [0x11, 0x00]),       // frame cycle
            (0x0C, [0x00, 0x00]),       // CPU interface
            (0x0F, [0x0F, 0x01]),       // oscillator
            (0x15, [0x00, 0x20]),       // VCI recycling
            (0x20, [0x00, 0x00]),       // RAM address X
            (0x21, [0x00, 0x00]),       // RAM address Y
            // GRAM area
            (0x30, [0x00, 0x00]),
            (0x31, [0x00, 0xDB]),
            (0x32, [0x00, 0x00]),
            (0x33, [0x00, 0x00]),
            (0x34, [0x00, 0xDB]),
            (0x35, [0x00, 0x00]),
            (0x36, [0x00, 0xAF]),
            (0x37, [0x00, 0x00]),
            (0x38, [0x00, 0xDB]),
            (0x39, [0x00, 0x00]),
            // gamma
            (0x50, [0x00, 0x00]),
            (0x51, [0x08, 0x08]),
            (0x52, [0x08, 0x0A]),
            (0x53, [0x00, 0x0A]),
            (0x54, [0x0A, 0x08]),
            (0x55, [0x08, 0x08]),
            (0x56, [0x00, 0x00]),
            (0x57, [0x0A, 0x00]),
            (0x58, [0x07, 0x10]),
            (0x59, [0x07, 0x10]),
        ] {
            di.write_raw(r, &v)?;
        }

        di.write_raw(0x07, &[0x00, 0x12])?;
        delay.delay_us(50_000);

        // DISP_CTRL1. Bit 2 of the low byte is REV.
        let low = 0b1_0011
            | match options.invert_colors {
                ColorInversion::Normal => 0,
                ColorInversion::Inverted => 0b100,
            };
        di.write_raw(0x07, &[0x10, low])?;
        delay.delay_us(50_000);

        Ok(SetAddressMode::from(options))
    }

    /// The stock model leaves this at the DCS default, which sends 0x01 --
    /// `DRIVER_OUTPUT_CTRL` on this chip, not a reset. 0x28 is the real one.
    fn software_reset<DI>(di: &mut DI) -> Result<(), DI::Error>
    where
        DI: Interface,
    {
        di.write_command(SoftResetILI9225)
    }

    // ---- everything below is upstream's behaviour, delegated -------------

    fn update_address_window<DI>(
        di: &mut DI,
        rotation: Rotation,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), DI::Error>
    where
        DI: Interface,
    {
        <ILI9225Rgb565 as Model>::update_address_window(di, rotation, sx, sy, ex, ey)
    }

    fn update_options<DI>(&self, di: &mut DI, options: &ModelOptions) -> Result<(), DI::Error>
    where
        DI: Interface,
    {
        ILI9225Rgb565.update_options(di, options)
    }

    fn write_memory_start<DI>(di: &mut DI) -> Result<(), DI::Error>
    where
        DI: Interface,
    {
        <ILI9225Rgb565 as Model>::write_memory_start(di)
    }

    fn sleep<DI, DELAY>(di: &mut DI, delay: &mut DELAY) -> Result<(), DI::Error>
    where
        DI: Interface,
        DELAY: DelayNs,
    {
        <ILI9225Rgb565 as Model>::sleep(di, delay)
    }

    fn wake<DI, DELAY>(di: &mut DI, delay: &mut DELAY) -> Result<(), DI::Error>
    where
        DI: Interface,
        DELAY: DelayNs,
    {
        <ILI9225Rgb565 as Model>::wake(di, delay)
    }

    fn set_tearing_effect<DI>(
        di: &mut DI,
        tearing_effect: TearingEffect,
        options: &ModelOptions,
    ) -> Result<(), DI::Error>
    where
        DI: Interface,
    {
        <ILI9225Rgb565 as Model>::set_tearing_effect(di, tearing_effect, options)
    }

    fn set_vertical_scroll_region<DI>(
        di: &mut DI,
        top_fixed_area: u16,
        bottom_fixed_area: u16,
    ) -> Result<(), DI::Error>
    where
        DI: Interface,
    {
        <ILI9225Rgb565 as Model>::set_vertical_scroll_region(di, top_fixed_area, bottom_fixed_area)
    }

    fn set_vertical_scroll_offset<DI>(di: &mut DI, offset: u16) -> Result<(), DI::Error>
    where
        DI: Interface,
    {
        <ILI9225Rgb565 as Model>::set_vertical_scroll_offset(di, offset)
    }
}