use crate::bus::{Bus, VRAM};

enum FetcherState {
    FetchTileNumber,
    ReadTileData0,
    ReadTileData1,
    PushToFIFO,
}

pub struct Fetcher {
    ticks: usize,
    tile_id: u16,
    x_position_counter: u8,
    pixel_data: [u8; 8],
    state: FetcherState
}

impl Fetcher  {
    pub fn new(bus: &Bus) ->Fetcher {
        Fetcher {
            ticks: 0,
            tile_id: 0,
            x_position_counter: 0,
            pixel_data: [0; 8],
            state: FetcherState::FetchTileNumber,
        }
    }
    pub fn start(&mut self, x: u8){
        self.ticks = 0;
        self.tile_id = 0;
        self.x_position_counter = x;
        self.pixel_data.fill(0);
        self.state = FetcherState::FetchTileNumber
    }
    pub fn tick(&mut self, bus: &mut Bus) {
        self.ticks += 1;
        if self.ticks % 2 == 2 { return; }

        match self.state {
            FetcherState::FetchTileNumber => self.fetch_tile_number(bus),
            FetcherState::ReadTileData0 => self.read_tile_data0(bus),
            FetcherState::ReadTileData1 => self.read_tile_data1(bus),
            FetcherState::PushToFIFO => self.push_to_fifo(bus),
        }
    }

    fn fetch_tile_number(&mut self, bus: &Bus){
        self.tile_id = match (bus.get_ldlc_window_enable(), bus.get_ly() >= bus.get_wy(), self.x_position_counter >= bus.get_wx()) {
            (true, true, true) => {
                self.x_position_counter as u16 + 32 * (((bus.get_ly() - bus.get_wy()) & 0xFF) as u16 / 8)
            },
            (_, _, _) => {
                self.x_position_counter as u16 + ((bus.get_scx() as u16 / 8) & 0x1f) + 32 * (((bus.get_ly().wrapping_add(bus.get_scy())) & 0xFF) as u16 / 8)
            },
        };
        self.pixel_data.fill(0);
        self.state = FetcherState::ReadTileData0
    }
    fn read_tile_data0(&mut self, bus: &Bus){
        let offset = match bus.get_ldlc_bg_window_tiles() {
            true => 0x8000 + 16 * self.tile_id,
            false => 0x9000u16.wrapping_add_signed(self.tile_id as i16 * 16)
        };
        let data = bus.get(offset + (bus.get_ly() as u16 % 8) * 2);
        for bit in 0..=7 {
            self.pixel_data[bit] = (data >> bit) & 1
        }

        self.state = FetcherState::ReadTileData1
    }
    fn read_tile_data1(&mut self, bus: &Bus){
        let offset = match bus.get_ldlc_bg_window_tiles() {
            true => 0x8000 + 16 * self.tile_id,
            false => 0x9000u16.wrapping_add_signed(self.tile_id as i16 * 16)
        };
        let data = bus.get(offset + (bus.get_ly() as u16 % 8) * 2 +1);
        for bit in 0..=7 {
            self.pixel_data[bit] |= ((data >> bit) & 1) << 1
        }
        self.state = FetcherState::PushToFIFO
    }
    fn push_to_fifo(&mut self, bus: &mut Bus){
        if bus.fifo.len() <= 8 {
            for i in (0..=7).rev() {
                bus.fifo.push(self.pixel_data[i]);
            }
            self.x_position_counter += 1;
            self.state = FetcherState::FetchTileNumber;
        }
    }
}