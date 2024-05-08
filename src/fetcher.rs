use crate::bus::{Bus, VRAM};

enum FetcherState {
    FetchTileNumber,
    ReadTileData0,
    ReadTileData1,
    PushToFIFO,
    ReadTileID
}

pub struct Fetcher {
    ticks: usize,
    tile_number: u8,
    x_position_counter: u8,
    pixel_data: [u8; 8],
    state: FetcherState
}

impl Fetcher  {
    pub(crate) fn new(bus: &Bus) ->Fetcher {
        Fetcher {
            ticks: 0,
            tile_number: 0,
            x_position_counter: 0,
            pixel_data: [0; 8],
            state: FetcherState::FetchTileNumber,
        }
    }
    pub fn tick(&mut self, bus: &mut Bus) {
        self.ticks += 1;
        if self.ticks % 2 == 2 { return; }

        match self.state {
            FetcherState::FetchTileNumber => self.fetch_tile_number(bus),
            FetcherState::ReadTileData0 => self.read_tile_data0(bus),
            FetcherState::ReadTileData1 => self.read_tile_data1(bus),
            FetcherState::PushToFIFO => self.push_to_fifo(bus),
            FetcherState::ReadTileID => { }
        }
    }

    fn fetch_tile_number(&mut self, bus: &Bus){
        self.tile_number = 0;
        self.pixel_data.fill(0);
        self.state = FetcherState::ReadTileData0
    }
    fn read_tile_data0(&mut self, bus: &Bus){
    }
    fn read_tile_data1(&mut self, bus: &Bus){
    }
    fn push_to_fifo(&mut self, bus: &mut Bus){
    }
}