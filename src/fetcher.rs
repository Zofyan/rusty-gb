use crate::bus::{Bus, VRAM};
use crate::fetcher::FetcherState::{PushToFIFO, ReadTileData0, ReadTileData1};
use crate::ppu::OAM;

enum FetcherState {
    ReadTileID,
    ReadTileData0,
    ReadTileData1,
    PushToFIFO,
}

pub struct Fetcher {
    ticks: usize,
    tile_id: u8,
    tile_index: u8,
    line_index: u8,
    tile_line: u8,
    map_address: u16,
    pixel_data: [u8; 16],
    oams: Vec<OAM>,
    fifo_bg: Vec<u8>,
    fifo_sprite: Vec<u8>,
    state: FetcherState
}

impl Fetcher  {
    pub fn new(bus: &Bus) ->Fetcher {
        Fetcher {
            ticks: 0,
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
        }
    }
    pub fn tick(&mut self, bus: &mut Bus) {
        self.ticks += 1;
        if self.ticks % 2 == 2 { return; }

        match self.state {
            FetcherState::ReadTileID => self.fetch_tile_number(bus),
            FetcherState::ReadTileData0 | ReadTileData1 => self.read_tile_data(bus),
            FetcherState::PushToFIFO => self.push_to_fifo(bus),
        }
    }

    pub fn reset(&mut self, mmap_addr: u16, tile_line: u8, bus: &Bus){
        self.tile_index = bus.get_scx() % 8;
        self.line_index = (bus.get_scy() / 8 + bus.get_ly() / 8) % 32;
        self.map_address = mmap_addr;
        self.tile_line = tile_line;
        self.state = FetcherState::ReadTileID;
        self.fifo_bg.clear();

    }
    fn fetch_tile_number(&mut self, bus: &Bus){
        /*self.tile_id = match (bus.get_ldlc_window_enable(), bus.get_ly() >= bus.get_wy(), self.x_position_counter >= bus.get_wx()) {
            (true, true, true) => {
                self.x_position_counter as u16 + 32 * (((bus.get_ly() - bus.get_wy()) & 0xFF) as u16 / 8)
            },
            (_, _, _) => {
                self.x_position_counter as u16 + ((bus.get_scx() as u16 / 8) & 0x1f) + 32 * (((bus.get_ly().wrapping_add(bus.get_scy())) & 0xFF) as u16 / 8)
            },
        };*/
        self.tile_id = bus.get(self.map_address + self.tile_index as u16 + self.line_index as u16 * 32);
        self.pixel_data.fill(0);
        self.state = FetcherState::ReadTileData0
    }
    fn read_tile_data(&mut self, bus: &Bus){
        let offset = match bus.get_ldlc_bg_window_tiles() {
            true => 0x8000 + 16 * self.tile_id as u16,
            false => 0x9000u16.wrapping_add_signed(self.tile_id as i16 * 16)
        };
        let offset2 = match self.state {
            ReadTileData0 => 0,
            ReadTileData1 => 1,
            _ => panic!("invalid state")
        };
        let data = bus.get(offset + offset2 + (bus.get_ly() as u16 % 8) * 2);
        for bit in 0..=7 {
            match self.state {
                ReadTileData0 => {
                    self.pixel_data[bit] = (data >> bit) & 1
                }
                ReadTileData1 => {
                    self.pixel_data[bit] |= (data >> bit) << 1
                }
                _ => {
                    panic!("invalid fetch state");
                }
            }
        }

        self.state =
            match self.state {
                ReadTileData0 => ReadTileData1,
                ReadTileData1 => PushToFIFO,
                _ => {
                    panic!("invalid fetch state");
                }
            }
    }
    fn push_to_fifo(&mut self, bus: &mut Bus){
        if bus.fifo.len() <= 8 {
            for i in (0..=7).rev() {
                bus.fifo.push(self.pixel_data[i]);
            }
            self.x_position_counter += 1;
            self.state = FetcherState::ReadTileID;
        }
    }
}