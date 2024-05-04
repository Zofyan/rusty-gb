use std::cmp::PartialEq;
use crate::bus::{Bus, VRAM};
use crate::fetcher::FetcherState::{PushToFIFO, ReadTileData0, ReadTileData1, ReadTileID};

enum FetcherState {
    ReadTileData0,
    ReadTileData1,
    PushToFIFO,
    ReadTileID
}

pub struct Fetcher {
    ticks: usize,
    tile_id: u8,
    tile_index: u8,
    map_address: u16,
    tile_line: u8,
    pixel_data: [u8; 8],
    state: FetcherState
}

impl Fetcher  {
    pub(crate) fn new() ->Fetcher {
        Fetcher {
            ticks: 0,
            tile_id: 0,
            tile_index: 0,
            map_address: 0,
            tile_line: 0,
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
            ReadTileData0 => self.readTileData0(bus),
            ReadTileData1 => self.readTileData1(bus),
            PushToFIFO => self.pushToFIFO(bus),
            ReadTileID => self.readTileID(bus)
        }
    }

    fn readTileData0(&mut self, bus: &Bus){
        let offset = VRAM + (self.tile_id as u16 * 16);
        let address = offset + self.tile_line as u16 * 2;
        let value = bus.get(address);
        for i in 0..=7 {
            self.pixel_data[i] = (value >> i) & 1;
        }
        self.state = ReadTileData1
    }
    fn readTileData1(&mut self, bus: &Bus){
        let offset = VRAM + (self.tile_id as u16 * 16);
        let address = offset + self.tile_line as u16 * 2;
        let value = bus.get(address + 1);
        for i in 0..=7 {
            self.pixel_data[i] |= ((value >> i) & 1) << 1;
        }
        self.state = PushToFIFO
    }
    fn pushToFIFO(&mut self, mut bus: &mut Bus){
        if bus.fifo.len() <= 8 {
            for i in (0..=7).rev() {
                bus.fifo.push(self.pixel_data[i]);
            }
            self.tile_index.overflowing_add(1);
            self.state = ReadTileID;
        }
    }
    fn readTileID(&mut self, mut bus: &Bus){
        self.tile_id = bus.get(self.map_address + self.tile_index as u16);
        self.pixel_data.fill(0);
        self.state = ReadTileData0
    }
}