use crate::bus::{Bus, VRAM};
use crate::ppu::OAM;
use crate::window_fetcher::WindowFetcherState::{
    PushToFIFO, ReadTileData0, ReadTileData1, ReadTileID,
};

enum WindowFetcherState {
    ReadTileData0,
    ReadTileData1,
    PushToFIFO,
    ReadTileID,
}

pub struct WindowFetcher {
    ticks: usize,
    tile_index: u8,
    tile_id: u8,
    map_address: u16,
    tile_line: u8,
    line_index: u8,
    pixel_data: [u8; 16],
    oams: Vec<OAM>,
    pub(crate) fifo_bg: Vec<u8>,
    fifo_sprite: Vec<u8>,
    state: WindowFetcherState,
}

impl WindowFetcher {
    pub fn new() -> WindowFetcher {
        WindowFetcher {
            ticks: 0,
            tile_index: 0,
            map_address: 0,
            tile_line: 0,
            tile_id: 0,
            pixel_data: [0; 16],
            oams: vec![],
            fifo_bg: vec![],
            fifo_sprite: vec![],
            state: ReadTileID,
            line_index: 0,
        }
    }
    pub fn tick(&mut self, bus: &mut Bus) {
        self.ticks += 1;
        if self.ticks < 2 {
            //return;
        }
        self.ticks = 0;

        match self.state {
            ReadTileData0 | ReadTileData1 => self.read_tile_data(bus),
            PushToFIFO => self.push_to_fifo(bus),
            ReadTileID => self.read_tile_id(bus),
        }
    }

    fn read_tile_data(&mut self, bus: &Bus) {
        let offset = match bus.get_ldlc_window_tilemap() {
            true => 0x8000 + self.tile_id as u16 * 16,
            false => 0x9000u16.wrapping_add_signed(self.tile_id as i16 * 16),
        };
        let offset2 = match self.state {
            ReadTileData0 => 0,
            ReadTileData1 => 1,
            _ => unreachable!(),
        };
        let address = offset + offset2 + self.tile_line as u16 * 2;
        let value = bus.get(address);
        for bit in 0..=7 {
            match self.state {
                ReadTileData0 => self.pixel_data[bit] = (value >> bit) & 1,
                ReadTileData1 => self.pixel_data[bit] |= ((value >> bit) & 1 ) << 1,
                _ => {
                    panic!("invalid fetch state");
                }
            }
        }

        self.state = match self.state {
            ReadTileData0 => ReadTileData1,
            ReadTileData1 => PushToFIFO,
            _ => {
                panic!("invalid fetch state");
            }
        }
    }
    fn push_to_fifo(&mut self, bus: &mut Bus) {
        if self.fifo_bg.len() <= 8 {
            for i in (0..=7).rev() {
                self.fifo_bg.push(self.pixel_data[i]);
            }
            self.tile_index = (self.tile_index + 1) % 32;
            self.state = ReadTileID;
        }
    }
    fn read_tile_id(&mut self, bus: &Bus) {
        self.tile_id = bus.get(self.map_address + self.tile_index as u16);
        self.pixel_data.fill(0);
        self.state = ReadTileData0
    }

    pub fn reset(&mut self, mmap_addr: u16, tile_line: u8){
        self.tile_index = 0;
        self.map_address = mmap_addr;
        self.tile_line = tile_line;
        self.state = ReadTileID;
        self.fifo_bg.clear();
    }
}
