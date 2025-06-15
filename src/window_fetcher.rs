use crate::bus::{Bus, VRAM};
use crate::ppu::OAM;
use crate::window_fetcher::WindowFetcherState::{PushToFIFO, ReadTileData0, ReadTileID};

enum WindowFetcherState {
    ReadTileData0,
    ReadTileData1,
    PushToFIFO,
    ReadTileID,
}

pub struct WindowFetcher {
    ticks: usize,
    pub(crate) tile_map: u16,
    tile_data: u16,
    tile_index: u8,
    tile_id: u8,
    map_address: u16,
    tile_line: u8,
    line_index: u8,
    pub tiles_set: bool,
    pixel_data: [u8; 16],
    oams: Vec<OAM>,
    pub fifo_bg: Vec<u8>,
    fifo_sprite: Vec<u8>,
    state: WindowFetcherState,
}

impl WindowFetcher {
    pub fn new() -> WindowFetcher {
        WindowFetcher {
            ticks: 0,
            tile_map: 0,
            tile_data: 0,
            tile_index: 0,
            map_address: 0,
            tile_line: 0,
            tile_id: 0,
            pixel_data: [0; 16],
            oams: vec![],
            fifo_bg: Vec::with_capacity(16),
            fifo_sprite: Vec::with_capacity(16),
            state: ReadTileID,
            line_index: 0,
            tiles_set: true,
        }
    }
    pub fn tick(&mut self, bus: &mut Bus) {
        match self.state {
            ReadTileData0 => self.read_tile_data(bus),
            PushToFIFO => self.push_to_fifo(bus),
            _ => panic!("should not be possible")
        }
    }

    fn read_tile_data(&mut self, bus: &Bus) {
        let offset = match bus.get_ldlc_bg_window_tiles() {
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

        for bit in [0,1,2,3,4,5,6,7] {
            self.fifo_bg.push((value1 >> bit) & 1 | ((value2 >> bit) & 1 ) << 1);
        }

        self.state = PushToFIFO;
    }
    fn push_to_fifo(&mut self, bus: &mut Bus) {
        if self.fifo_bg.len() <= 8 {
            self.tile_index = (self.tile_index + 1) % 32;
            self.read_tile_id(bus);
        }
    }
    fn read_tile_id(&mut self, bus: &Bus) {
        self.tile_id = bus.memory.get(self.map_address + self.tile_index as u16);
        self.pixel_data.fill(0);
        self.state = ReadTileData0
    }

    pub fn reset(&mut self, mmap_addr: u16, tile_line: u8, bus: &mut Bus) {
        self.tile_index = 0;
        self.map_address = mmap_addr;
        self.tile_line = tile_line;
        self.read_tile_id(bus);
        self.state = ReadTileData0;
        self.fifo_bg.clear();
    }
}
