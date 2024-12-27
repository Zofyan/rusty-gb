use crate::bus::{Bus, VRAM};
use crate::ppu::OAM;


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
    pixel_data: [u8; 16],
    oams: Vec<OAM>,
    pub fifo_bg: Vec<u8>,
    fifo_sprite: Vec<u8>,
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
            pixel_data: [0; 16],
            oams: vec![],
            fifo_bg: vec![],
            fifo_sprite: vec![],
            state: FetcherState::ReadTileID,
            line_index: 0,
            tiles_set: true,
        }
    }
    pub fn tick(&mut self, bus: &mut Bus) {
        self.ticks += 1;
        if self.ticks < 2 {
            return;
        }
        self.ticks = 0;

        match self.state {
            FetcherState::ReadTileData0 | FetcherState::ReadTileData1 => self.read_tile_data(bus),
            FetcherState::PushToFIFO => self.push_to_fifo(bus),
            FetcherState::ReadTileID => self.read_tile_id(bus),
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
        let offset2 = match self.state {
            FetcherState::ReadTileData0 => 0,
            FetcherState::ReadTileData1 => 1,
            _ => unreachable!(),
        };
        let address = offset + offset2 + self.tile_line as u16 * 2;
        let value = bus._get(address);
        for bit in 0..=7 {
            match self.state {
                FetcherState::ReadTileData0 => self.pixel_data[7 - bit] = (value >> bit) & 1,
                FetcherState::ReadTileData1 => self.pixel_data[7 - bit] |= ((value >> bit) & 1 ) << 1,
                _ => {
                    panic!("invalid fetch state");
                }
            }
        }

        self.state = match self.state {
            FetcherState::ReadTileData0 => FetcherState::ReadTileData1,
            FetcherState::ReadTileData1 => FetcherState::PushToFIFO,
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
            self.state = FetcherState::ReadTileID;
        }
    }
    fn read_tile_id(&mut self, bus: &Bus) {
        self.tile_id = bus._get(self.map_address + self.tile_index as u16 + self.line_index as u16 * 32);
        self.pixel_data.fill(0);
        self.state =FetcherState:: ReadTileData0
    }

    pub fn reset(&mut self, mmap_addr: u16, tile_line: u8, bus: &Bus){
        self.tile_index = bus.get_scx() / 8;
        self.line_index = (bus.get_scy() / 8 + bus.get_ly() / 8) % 32;
        self.map_address = mmap_addr;
        self.tile_line = tile_line;
        self.state = FetcherState::ReadTileID;
        self.fifo_bg.clear();
    }
}
