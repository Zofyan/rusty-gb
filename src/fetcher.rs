use crate::bus::{Bus, VRAM};

/// Spreads the 8 bits of `b` so bit `i` lands at bit `2i`.
#[inline]
fn spread(b: u8) -> u16 {
    let mut x = b as u16;
    x = (x | (x << 4)) & 0x0F0F;
    x = (x | (x << 2)) & 0x3333;
    (x | (x << 1)) & 0x5555
}

/// Packs one 8-pixel tile row from its two bitplanes into 2 bits per pixel,
/// leftmost pixel in the high bits.
///
/// The straightforward loop over the 8 bits compiled to 35 Thumb instructions
/// ending in 8 byte stores, which the consumer then had to load back one at a
/// time. This is 23 instructions and leaves the whole row in one register.
#[inline]
pub fn decode_row(lo: u8, hi: u8) -> u16 {
    spread(lo) | (spread(hi) << 1)
}

/// Reverses the order of the eight 2-bit pixels, for a horizontally flipped
/// sprite. `reverse_bits` is a single `rbit` on Cortex-M33; it also swaps the
/// two bits within each pixel, which the mask-and-shift undoes.
#[inline]
pub fn flip_row(row: u16) -> u16 {
    let r = row.reverse_bits();
    ((r & 0x5555) << 1) | ((r >> 1) & 0x5555)
}

/// Number of pixels a packed row holds.
pub const ROW_PIXELS: usize = 8;

enum FetcherState {
    ReadTileData0,
    ReadTileData1,
    PushToFIFO,
    ReadTileID,
}

pub struct Fetcher {
    ticks: usize,
    pub(crate) tile_map: u16,
    tile_data: u16,
    tile_index: u8,
    tile_id: u8,
    map_address: u16,
    tile_line: u8,
    line_index: u8,
    pub tiles_set: bool,
    /// 8 pixels at 2 bits each, leftmost in the high bits.
    pub fifo_bg: u16,
    pub fifo_bg_size: usize,
    state: FetcherState,
}

impl Fetcher {
    pub fn new() -> Fetcher {
        Fetcher {
            ticks: 0,
            tile_map: 0,
            tile_data: 0,
            tile_index: 0,
            map_address: 0,
            tile_line: 0,
            tile_id: 0,
            fifo_bg: 0,
            fifo_bg_size: 0,
            state: FetcherState::ReadTileID,
            line_index: 0,
            tiles_set: true,
        }
    }
    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    pub fn tick(&mut self, bus: &mut Bus) {
        match self.state {
            FetcherState::ReadTileData0 => self.read_tile_data(bus),
            FetcherState::PushToFIFO => self.push_to_fifo(bus),
            _ => panic!("should not be possible")
        }
    }

    fn read_tile_data(&mut self, bus: &Bus) {
        self.tiles_set = bus.get_ldlc_bg_window_tiles();
        let offset = match self.tiles_set {
            true => 0x8000 + self.tile_id as u16 * 16,
            false => {
                if self.tile_id <= 127 {
                    0x9000 + self.tile_id as u16 * 16
                } else {
                    0x8000 + self.tile_id as u16 * 16
                }
            }
        };
        let address = offset + self.tile_line as u16 * 2;
        let value1 = bus.memory.get(address);
        let value2 = bus.memory.get(address + 1);

        // The consumer drains the FIFO on every tick, so a row never lands on
        // top of leftover pixels.
        debug_assert_eq!(self.fifo_bg_size, 0);
        self.fifo_bg = decode_row(value1, value2);
        self.fifo_bg_size = ROW_PIXELS;

        self.state = FetcherState::PushToFIFO;
    }
    fn push_to_fifo(&mut self, bus: &mut Bus) {
        if self.fifo_bg_size <= 8 {
            self.tile_index = (self.tile_index + 1) % 32;
            self.read_tile_id(bus);
        }
    }
    fn read_tile_id(&mut self, bus: &Bus) {
        self.tile_id = bus.memory.get(self.map_address + self.tile_index as u16 + self.line_index as u16 * 32);
        self.state = FetcherState:: ReadTileData0
    }

    pub fn reset(&mut self, mmap_addr: u16, tile_line: u8, bus: &Bus){
        self.tile_index = bus.get_scx() / 8;
        self.line_index = (bus.get_scy() / 8 + bus.get_ly() / 8) % 32;
        self.map_address = mmap_addr;
        self.tile_line = tile_line;
        self.read_tile_id(bus);
        self.state = FetcherState::ReadTileData0;
        self.fifo_bg_size = 0;
    }
}
