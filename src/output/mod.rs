pub mod dummy;
/// Software framebuffer. Needs only `alloc`, so it builds for both targets.
pub mod lcdd;

// Host-only backends: `lcd` pulls in winit/pixels and `terminal` pulls in
// ratatui/colored, none of which build for `thumbv8m`. Gated rather than
// commented out so that `cargo check` on the host still covers them.
#[cfg(not(target_os = "none"))]
pub mod lcd;
#[cfg(not(target_os = "none"))]
pub mod terminal;

#[cfg(target_os = "none")]
mod panel;
#[cfg(target_os = "none")]
pub mod spi;

use alloc::string::String;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

/// Colour of a packed pixel, as handed to [`Output::write_line`].
pub const PX_COLOR: u8 = 0b11;
/// Set when the pixel uses the second (OBP1) palette.
pub const PX_PALETTE: u8 = 0b100;
/// Set when the pixel came from a sprite rather than the background.
pub const PX_SPRITE: u8 = 0b1000;

pub trait Output {
    /// Hands over one finished scanline of [`SCREEN_WIDTH`] packed pixels.
    ///
    /// This is per-line rather than per-pixel on purpose: per-pixel dispatch
    /// measured about 14% of emulator runtime, and a backend expands palettes
    /// far more cheaply in a tight loop than interleaved with pixel fetching.
    fn write_line(&mut self, _y: u16, _line: &[u8; SCREEN_WIDTH]) {}
    fn refresh(&mut self) -> bool {
        true
    }
    fn set_diagnostics(&mut self, _diagnostics: String) {}
}
