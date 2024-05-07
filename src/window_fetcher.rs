use crate::bus::{Bus, VRAM};
use crate::window_fetcher::WindowFetcherState::{PushToFIFO, ReadTileData0, ReadTileData1, ReadTileID};

enum WindowFetcherState {
    ReadTileData0,
    ReadTileData1,
    PushToFIFO,
    ReadTileID
}

pub struct WindowFetcher {
    ticks: usize,
    tile_id: u8,
    tile_index: u8,
    map_address: u16,
    line_id: u8,
    line_index: u8,
    pixel_data: [u8; 8],
    state: FetcherState
}

impl WindowFetcher  {
    pub(crate) fn new(bus: &Bus) -> WindowFetcher {
        WindowFetcher {
            ticks: 0,
            tile_id: 0,
            tile_index: 0,
            map_address: 0,
            line_id: 0,
            line_index: 0,
            pixel_data: [0; 8],
            state: ReadTileData0,
        }
    }
    pub fn tick(&mut self, bus: &mut Bus) {
        self.ticks += 1;
        if self.ticks < 2 {
            return;
        }
        self.ticks = 0;

        match self.state {
            ReadTileData0 => self.read_tile_data0(bus),
            ReadTileData1 => self.read_tile_data1(bus),
            PushToFIFO => self.push_to_fifo(bus),
            ReadTileID => self.read_tile_id(bus)
        }
    }

    fn read_tile_data0(&mut self, bus: &Bus){
        let offset = match bus.get_ldlc_window_tilemap() {
            true => 0x8000 + self.tile_id as u16 * 16,
            false => 0x9000 + self.tile_id as u16 * 16,
        };
        let address = offset + self.line_id as u16 * 2;
        let value = bus.get(address);
        for i in 0..=7 {
            self.pixel_data[i] = (value >> i) & 1;
        }
        self.state = ReadTileData1
    }
    fn read_tile_data1(&mut self, bus: &Bus){
        let offset = match bus.get_ldlc_window_tilemap() {
            true => 0x8000 + self.tile_id as u16 * 16,
            false => 0x9000 + self.tile_id as u16 * 16,
        };
        let address = offset + self.line_id as u16 * 2;
        let value = bus.get(address + 1);
        for i in 0..=7 {
            self.pixel_data[i] |= ((value >> i) & 1) << 1;
        }
        self.state = PushToFIFO
    }
    fn push_to_fifo(&mut self, bus: &mut Bus){
        if bus.fifo.len() <= 8 {
            for i in (0..=7).rev() {
                bus.fifo.push(self.pixel_data[i]);
            }
            self.tile_index = self.tile_index + 1 % 32;
            self.state = ReadTileID;
        }
    }
    fn read_tile_id(&mut self, bus: &Bus){
        self.tile_id = bus.get(self.map_address + self.tile_index as u16);
        self.pixel_data.fill(0);
        self.state = ReadTileData0
    }
}