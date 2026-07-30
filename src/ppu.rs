use crate::bus::{Bus, OAM};
use crate::fetcher::{decode_row, flip_row, Fetcher};
use crate::output::Output;
use crate::output::{PX_COLOR, PX_PALETTE, PX_SPRITE, SCREEN_WIDTH};
use crate::window_fetcher::WindowFetcher;
use bitfield::Bit;
use core::cmp::min;

const PPU_LINE_LENGTH: usize = 456;
pub struct OAM {
    address: u16,
    y: u8,
    x: u8,
    tile_index: u8,
    palette: bool,
    flip_x: bool,
    flip_y: bool,
    priority: bool,
    /// This row of the sprite, 2 bits per pixel, leftmost in the high bits.
    data: u16,
}

impl OAM {
    pub fn new(index: usize, bus: &Bus) -> OAM {
        let address = OAM as u16 + (index as u16) * 4;
        let mut oam = OAM::empty();
        oam.y = bus.memory.get(address);
        oam.address = address;
        oam
    }
    pub fn init(&mut self, bus: &Bus) {
        self.x = bus.memory.get(self.address + 1);
        self.tile_index = bus.memory.get(self.address + 2);
        let tmp = bus.memory.get(self.address + 3);
        self.palette = tmp.bit(4);
        self.flip_x = tmp.bit(5);
        self.flip_y = tmp.bit(6);
        self.priority = tmp.bit(7);
    }
    pub fn set(&mut self, val: u8, index: u8) {
        match index {
            0 => self.y = val,
            1 => self.x = val,
            2 => self.tile_index = val,
            3 => {
                self.palette = val.bit(4);
                self.flip_x = val.bit(5);
                self.flip_y = val.bit(6);
                self.priority = val.bit(7);
            },
            _ => { panic!("no oam") }
        }
    }
    pub fn empty() -> OAM {
        OAM {
            address: 0xDF,
            y: 0xDF,
            x: 0xDF,
            tile_index: 0,
            palette: true,
            flip_x: true,
            flip_y: true,
            priority: true,
            data: 0,
        }
    }
}

impl Clone for OAM {
    fn clone(&self) -> Self {
        OAM {
            address: 0,
            y: 0,
            x: 0,
            tile_index: 0,
            palette: false,
            flip_x: false,
            flip_y: false,
            priority: false,
            data: 0,
        }
    }
}

impl Copy for OAM {}

#[derive(Clone)]
pub enum PpuState {
    HBlank = 0,
    VBlank = 1,
    OAMFetch = 2,
    PixelTransfer = 3,
}

/// Screen x range covered by the sprite line buffer. Sprite x is offset by 8
/// (an OAM x of 0 is 8 pixels left of the screen), so index = screen_x + 8
/// and the widest visible sprite starts at index 167.
const SPRITE_LINE_LEN: usize = 176;

/// Packed sprite pixel: bits 0-1 colour (0 = no sprite here), bit 2 palette,
/// bit 3 bg-over-obj priority.
const SPR_COLOR: u8 = 0b11;
const SPR_PALETTE: u8 = 0b100;
const SPR_PRIORITY: u8 = 0b1000;

pub struct Ppu {
    pub ticks: usize,
    pub state: PpuState,
    /// Indices into `bus.oams` of the sprites on this line, x-ascending.
    oambuffer: [u8; 10],
    oambuffer_len: usize,
    /// The line's sprites rasterised once per scanline, so pixel transfer is a
    /// single indexed load instead of a scan over up to 10 sprites.
    sprite_line: [u8; SPRITE_LINE_LEN],
    /// The composited scanline, handed to the output once the line completes.
    line: [u8; SCREEN_WIDTH],
    x: i16,
    x_shift: i16,
    y: i16,
    y_shift: i16,
    window_y_hit: bool,
    fetcher: Fetcher,
    window_fetcher: WindowFetcher,
    target_ticks: usize,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            ticks: PPU_LINE_LENGTH,
            target_ticks: PPU_LINE_LENGTH - 80,
            state: PpuState::OAMFetch,
            oambuffer: [0; 10],
            oambuffer_len: 0,
            sprite_line: [0; SPRITE_LINE_LEN],
            line: [0; SCREEN_WIDTH],
            x: 0,
            x_shift: 0,
            y: 0,
            y_shift: 0,
            window_y_hit: false,
            fetcher: Fetcher::new(),
            window_fetcher: WindowFetcher::new(),
        }
    }
    fn set_ppu_state(&mut self, bus: &mut Bus, state: PpuState) {
        bus.ppu_state = state.clone();
        let val = (bus.registers.lcds & 0b11111100) | state.clone() as u8;
        self.state = state.clone();
        bus.set(0xFF41, val);
        self.target_ticks = match state {
            PpuState::OAMFetch => PPU_LINE_LENGTH - 80,
            PpuState::PixelTransfer => self.target_ticks - 172,
            PpuState::HBlank => 0,
            PpuState::VBlank => 0,
        };
        self.ticks = match state {
            PpuState::OAMFetch => 456,
            PpuState::PixelTransfer => self.ticks,
            PpuState::HBlank => self.ticks,
            PpuState::VBlank => 456,
        }
    }

    fn dma_tranfer(&self, bus: &mut Bus) {
        let truncated_dma_address = bus.dma_address & 0xFF;
        let index = truncated_dma_address as usize / 4;
        // Via `bus`, not `memory`: the DMA source register can name a ROM page,
        // which no longer has a copy in RAM.
        let byte = bus.get(bus.dma_address);
        bus.oams[index].set(byte, (bus.dma_address & 0b11) as u8);
        if truncated_dma_address == 0x9F {
            bus.dma_address = 0;
        } else {
            bus.dma_address += 1;
        }
    }

    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    pub fn tick<O: Output>(&mut self, bus: &mut Bus, output: &mut O, mut ticks: usize) {
        while bus.dma_address != 0 {
            self.dma_tranfer(bus);
        }
        while ticks > 0 {
            let consumed_ticks = match self.state {
                PpuState::OAMFetch => {
                    self.oam_fetch(bus, ticks)
                }
                PpuState::PixelTransfer => {
                    self.pixel_tranfer(bus, output, ticks)
                }
                PpuState::HBlank => {
                    self.hblank(bus, ticks)
                }
                PpuState::VBlank => {
                    self.vblank(bus, output, ticks)
                }
            };
            ticks -= consumed_ticks;
        }
    }

    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    fn oam_fetch(&mut self, bus: &mut Bus, ticks: usize) -> usize {
        let mut i = 0;
        if ticks * 4 >= self.ticks - self.target_ticks {
            i = (self.ticks - self.target_ticks) / 4;
            self.ticks -= i * 4;
        } else {
            self.ticks -= ticks * 4;
            return ticks;
        }

        self.window_y_hit |= bus.get_wy() == bus.get_ly();
        let tile_line = bus.get_ly() % 8;

        let mut tile_map_row_addr = match bus.get_ldlc_bg_tilemap() {
            true => 0x9C00,
            false => 0x9800,
        };
        self.fetcher.reset(tile_map_row_addr, tile_line, bus);

        tile_map_row_addr = match bus.get_ldlc_window_tilemap() {
            true => 0x9C00,
            false => 0x9800,
        } + ((bus.get_ly() - bus.memory.get(0xFF4A)) / 8) as u16 * 32;
        self.window_fetcher.reset(tile_map_row_addr, tile_line, bus);

        self.x_shift = (bus.get_scx() % 8) as i16;
        self.y_shift = (bus.get_scy() % 8) as i16;
        self.x = -self.x_shift;
        self.line.fill(0);

        self.oambuffer_len = 0;
        for i in 0..40 {
            if (bus.get_ly().wrapping_sub(bus.oams[i].y.wrapping_sub(16)))
                < (match bus.get_ldlc_obj_size() {
                    true => 16,
                    false => 8,
                })
            {
                //oam.x = oam.x.saturating_sub(8);

                // Refetched on every scanline, and deliberately not cached.
                //
                // `data` is one row of the sprite, and which row depends on LY,
                // so a per-sprite cache key cannot describe it. Guarding this on
                // `tile_index` alone meant a sprite kept whatever row was fetched
                // on the first line it appeared on and repeated it down all 8 (or
                // 16) of its lines, smearing every sprite vertically. The key
                // could be widened to the computed address, but that changes on
                // every line too, so the hit rate would be zero.
                //
                // The decode below is still hoisted out of the per-pixel path,
                // which is where the win in doing this once per line actually is.
                let mut offset = 0x8000;
                let obj_size = bus.get_ldlc_obj_size();
                if obj_size {
                    if bus.oams[i].flip_y {
                        offset += ((bus.oams[i].tile_index | 0x01) as u16) * 0x10;
                    } else {
                        offset += ((bus.oams[i].tile_index & 0xFE) as u16) * 0x10;
                    }
                } else {
                    offset += (bus.oams[i].tile_index as u16) * 0x10;
                }

                let mut addr = offset;
                if bus.oams[i].flip_y {
                    addr += ((8 - (bus.get_ly() + 16 - bus.oams[i].y)) * 2) as u16;
                } else {
                    addr += ((bus.get_ly() + 16 - bus.oams[i].y) * 2) as u16;
                }

                let tmp1 = bus.memory.get(addr);
                let tmp2 = bus.memory.get(addr + 1);
                bus.oams[i].data = decode_row(tmp1, tmp2);
                self.oambuffer[self.oambuffer_len] = i as u8;
                self.oambuffer_len += 1;

                // This was `11 - min(5, (self.x + scx) % 8)`, but `self.x` is set
                // to `-(scx % 8)` above and not touched in this loop, so the sum
                // is always a multiple of 8 and the penalty is always 11. The old
                // form cost 24 Thumb instructions (signed remainder needs a bias
                // correction, and `checked_add_unsigned().unwrap()` emits a panic
                // branch) to compute a constant.
                //
                // NB: the variable penalty this was reaching for is therefore not
                // implemented -- see the note in the commit message.
                debug_assert_eq!(self.x, -((bus.get_scx() % 8) as i16));
                self.target_ticks -= 11;

                if self.oambuffer_len >= 10 {
                    break;
                }
            }
        }

        self.rasterise_sprites(bus);

        self.set_ppu_state(bus, PpuState::PixelTransfer);
        i
    }

    /// Draws this line's sprites into `sprite_line` once, so pixel transfer costs
    /// one indexed load per pixel instead of a scan over the whole OAM buffer.
    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    fn rasterise_sprites(&mut self, bus: &Bus) {
        self.sprite_line.fill(0);
        if !bus.get_ldlc_obj_enable() {
            return;
        }

        // Stable insertion sort by x: on DMG the lower x wins, and equal x is
        // broken by the lower OAM index, which is the order we collected them in.
        for i in 1..self.oambuffer_len {
            let idx = self.oambuffer[i];
            let x = bus.oams[idx as usize].x;
            let mut j = i;
            while j > 0 && bus.oams[self.oambuffer[j - 1] as usize].x > x {
                self.oambuffer[j] = self.oambuffer[j - 1];
                j -= 1;
            }
            self.oambuffer[j] = idx;
        }

        // Draw x-descending so higher-priority (lower x) sprites overwrite.
        for i in (0..self.oambuffer_len).rev() {
            let oam = &bus.oams[self.oambuffer[i] as usize];
            let packed = ((oam.palette as u8) << 2) | ((oam.priority as u8) << 3);
            // Resolve the flip once per sprite rather than branching per pixel.
            let mut row = if oam.flip_x { flip_row(oam.data) } else { oam.data };
            for diff in 0..8usize {
                let sx = oam.x as usize + diff;
                if sx >= SPRITE_LINE_LEN {
                    break;
                }
                let p = (row >> 14) as u8;
                row <<= 2;
                if p != 0 {
                    self.sprite_line[sx] = p | packed;
                }
            }
        }
    }

    /// Blends the background pixel `bg` with this line's sprite buffer and
    /// stores the result at the current x, then advances x.
    #[inline]
    fn compose(&mut self, bg: u8) {
        let x = self.x;
        self.x += 1;
        if x < 0 || x as usize >= SCREEN_WIDTH {
            return;
        }

        // Sprite x is offset by 8: OAM x of 0 sits 8 pixels left of the screen.
        let spr = self.sprite_line[(x + 8) as usize];
        self.line[x as usize] = if spr & SPR_COLOR != 0 && (spr & SPR_PRIORITY == 0 || bg == 0) {
            (spr & (PX_COLOR | PX_PALETTE)) | PX_SPRITE
        } else {
            bg & PX_COLOR
        };
    }

    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    fn pixel_tranfer<O: Output>(&mut self, bus: &mut Bus, mut output: &mut O, ticks: usize) -> usize {
        let mut pixel = 255;
        let condition = self.window_y_hit && bus.get_ldlc_window_enable() && self.x + 7 >= bus.get_wx() as i16;

        let mut i = 0;
        while i < ticks {
            self.ticks -= 4;
            i += 1;

            if condition {
                self.window_fetcher.tick(bus);
                while self.window_fetcher.fifo_bg_size > 0 {
                    self.window_fetcher.fifo_bg_size -= 1;
                    // Leftmost pixel is in the high bits.
                    let p = (self.window_fetcher.fifo_bg >> 14) as u8;
                    self.window_fetcher.fifo_bg <<= 2;
                    self.compose(p);
                    pixel = p;
                }
            } else {
                self.fetcher.tick(bus);
                while self.fetcher.fifo_bg_size > 0 {
                    self.fetcher.fifo_bg_size -= 1;
                    let p = (self.fetcher.fifo_bg >> 14) as u8;
                    self.fetcher.fifo_bg <<= 2;
                    self.compose(p);
                    pixel = p;
                }
            };

            if pixel != 255 {
                self.target_ticks = self.target_ticks.saturating_sub(4);
            }

            if self.ticks <= self.target_ticks + 1 {
                break;
            }
        }

        if i == ticks && self.ticks > self.target_ticks {
            return ticks;
        }

        if self.ticks <= self.target_ticks + 1 {
            // The line is complete: hand it over in one call.
            output.write_line(bus.get_ly() as u16, &self.line);
            if bus.get_ldlc_stat_hblank_stat_int() {
                bus.set_int_request_lcd(true);
            }
            self.set_ppu_state(bus, PpuState::HBlank);
        }
        i
    }

    fn hblank(&mut self, bus: &mut Bus, ticks: usize) ->usize{
        let mut i = 0;
        if ticks * 4 >= self.ticks - self.target_ticks {
            i = (self.ticks - self.target_ticks) / 4;
            self.ticks -= i * 4;
        } else {
            self.ticks -= ticks * 4;
            return ticks;
        }

        bus.set_ly(bus.get_ly() + 1);

        if bus.get_ly() == bus.get_lyc() {
            bus.setb(false, false, 2, 0xFF41);
            if bus.get_ldlc_stat_lyc_ly_stat_int() {
                bus.set_int_request_lcd(true);
            }
        } else {
            bus.reset(false, false, 2, 0xFF41);
        }

        if bus.get_ly() == 144 {
            bus.set_int_request_vblank(true);
            if bus.get_ldlc_stat_vblank_stat_int() {
                bus.set_int_request_lcd(true);
            }
            self.set_ppu_state(bus, PpuState::VBlank);
        } else {
            self.set_ppu_state(bus, PpuState::OAMFetch);
        }
        i
    }

    fn vblank<O: Output>(&mut self, bus: &mut Bus, _: &mut O, ticks: usize) -> usize {
        let mut i = 0;
        if ticks * 4 >= self.ticks - self.target_ticks {
            i = (self.ticks - self.target_ticks) / 4;
            self.ticks -= i * 4;
        } else {
            self.ticks -= ticks * 4;
            return ticks;
        }

        if bus.get_ly() == 153 {
            self.window_y_hit = false;

            bus.set_ly(0);

            if bus.get_ldlc_stat_oam_stat_int() {
                bus.set_int_request_lcd(true);
            }

            self.set_ppu_state(bus, PpuState::OAMFetch);
        } else {
            bus.set_ly(bus.get_ly() + 1);
            self.set_ppu_state(bus, PpuState::VBlank);
        }

        if bus.get_ly() == bus.get_lyc() {
            bus.setb(false, false, 2, 0xFF41);
            if bus.get_ldlc_stat_lyc_ly_stat_int() {
                bus.set_int_request_lcd(true);
            }
        } else {
            bus.reset(false, false, 2, 0xFF41);
        }
        i
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::Bus;
    use crate::fetcher::decode_row;
    use crate::ppu::Ppu;
    use crate::rom::Rom;

    /// A sprite's `data` is one row, selected by LY, so it has to be refetched on
    /// every scanline the sprite covers.
    ///
    /// Regression test for an OAM cache keyed on `tile_index` alone: a sprite
    /// keeps the same tile across all eight of its lines, so the guard hit on
    /// lines 1-7 and replayed the row fetched for line 0, smearing every sprite
    /// vertically. Nothing in the blargg cpu_instrs suite looks at the PPU, so
    /// this went unnoticed.
    #[test]
    fn sprite_row_follows_scanline() {
        /// An OAM y of 16 puts the sprite's first row on LY 0.
        const SPRITE_TOP: u8 = 16;

        let mut bus = Bus::new(Rom::test());

        // LCDC 0: 8x8 sprites, and both tilemaps at 0x9800.
        bus.set(0xFF40, 0x00);
        // WY 0, so the window row arithmetic in `oam_fetch` cannot underflow.
        bus.memory.set(0xFF4A, 0);

        // Tile 0 with every row distinct: row r sets bit r of the low plane.
        for row in 0..8u8 {
            bus.memory.set(0x8000 + row as u16 * 2, 1u8 << row);
            bus.memory.set(0x8000 + row as u16 * 2 + 1, 0);
        }

        let sprite = &mut bus.oams[0];
        sprite.y = SPRITE_TOP;
        sprite.x = 8;
        sprite.tile_index = 0;
        sprite.flip_x = false;
        sprite.flip_y = false;
        sprite.priority = false;
        sprite.palette = false;

        for ly in 0..8u8 {
            // A fresh Ppu per line, so the tick accounting `oam_fetch` expects is
            // in its post-`new` state and the scan actually runs.
            let mut ppu = Ppu::new();
            bus.set_ly(ly);
            ppu.oam_fetch(&mut bus, 20);

            assert_eq!(
                bus.oams[0].data,
                decode_row(1u8 << ly, 0),
                "LY {ly} used the wrong sprite row"
            );
        }
    }
}
