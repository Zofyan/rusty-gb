use bitfield::Bit;
use crate::bus::Bus;
use crate::fetcher::Fetcher;
use crate::output::Output;

const LY: u16 = 0xFF44;
const LYC: u16 = 0xFF45;

struct OAM {
    address: u16,
    y: u8,
    x: u8,
    tile_index: u8,
    palette: bool,
    flip_x: bool,
    flip_y: bool,
    priority: bool,
}

impl OAM {
    pub fn new(address: u16, bus: &Bus) -> OAM {
        OAM {
            address,
            y: bus.get(address),
            x: bus.get(address + 1),
            tile_index: bus.get(address + 2),
            palette: bus.get(address + 3).bit(4),
            flip_x: bus.get(address + 3).bit(5),
            flip_y: bus.get(address + 3).bit(6),
            priority: bus.get(address + 3).bit(7),
        }
    }
    pub fn save(&self, bus: &mut Bus) {
        bus.set(self.address, self.y);
        bus.set(self.address + 1, self.x);
        bus.set(self.address + 2, self.tile_index);
        bus.set_bit(self.address + 3, 4, self.palette);
        bus.set_bit(self.address + 3, 5, self.flip_x);
        bus.set_bit(self.address + 3, 6, self.flip_y);
        bus.set_bit(self.address + 3, 7, self.priority);
    }
}

enum PpuState {
    OAMFetch,
    PixelTransfer,
    HBlank,
    VBlank,
}

struct Ppu {
    ticks: u16,
    state: PpuState,
}

impl Ppu {
    fn set_ppu_state(&mut self, bus: &mut Bus, state: u8) {
        let val = (bus.get(0xFF41) & 0b11111100) | state;
        bus.set(0xFF41, val);
    }
    fn tick(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
        self.ticks += 1;
        match self.state {
            PpuState::OAMFetch => {
                self.set_ppu_state(bus, 2);
                self.oam_fetch(bus, fetcher, output);
            }
            PpuState::PixelTransfer => {
                self.set_ppu_state(bus, 3);
                self.pixel_tranfer(bus, fetcher, output);
            }
            PpuState::HBlank => {
                self.set_ppu_state(bus, 0);
                self.hblank(bus, fetcher, output);
            }
            PpuState::VBlank => {
                self.set_ppu_state(bus, 1);
                self.vblank(bus, fetcher, output);
            }
        }
    }
    fn oam_fetch(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
        if self.ticks == 40 {
            self.state = PpuState::PixelTransfer;
            self.ticks = 0;

        let tile_line = bus.get_ly() % 8;
        let tile_map_row_addr = match bus.get_ldlc_bg_tilemap() {
            true => 0x9C00,
            false => 0x9800
        };
        fetcher.start(tile_map_row_addr, tile_line);

        let tile_map_row_addr = match bus.get_ldlc_bg_tilemap() {
            true => 0x9C00,
            false => 0x9800
        } + ((bus.get_ly() - bus.get_wy()) as u16 / 8) * 32;
        window_fetcher->start(tileMapRowAddr, tileLine);

        x_shift = bus->ppu_registers->scx % 8;

        x = (int16_t) -x_shift;

        ticks_pixeltransfer = 168;

        memset(oams_selection, 0, sizeof(oams_selection));
        uint8_t count = 0;
        for (int i = 0; i < 40; i++) {
            if ((uint8_t) (bus->ppu_registers->ly - (bus->sprites[i]->position_y - 16)) <
                (bus->ppu_registers->lcdc.obj_size ? 16 : 8)) {
                oams[i] = *bus->sprites[i];
                oams[i].position_x -= 8;
                oams_selection[i] = true;

                ticks_pixeltransfer += 11 - (uint8_t) fmin(5, (x + bus->ppu_registers->scx) % 8);
                if (count++ >= 10) break;
            }
        }
        }
    }
    fn pixel_tranfer(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
        if self.ticks < 160 {
            fetcher.tick(bus);
            let empty = bus.fifo.is_empty();
            if empty {
                let pixel = *bus.fifo.first().unwrap();
                bus.fifo.pop();
                output.write_pixel(self.ticks, bus.get(LY) as u16, pixel);
            }
        }
    }
    fn hblank(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
        bus.set_lyc(bus.get_ly() + 1);
        if bus.get_ly() == bus.get_lyc() {
            bus.setb(false, false, 2, 0xFF41);
            bus.set_int_request_lcd(true);
        } else {
            bus.reset(false, false, 2, 0xFF41);
        }
        if bus.get_ly() == 144 {
            bus.set_int_request_vblank(true);
            self.state = PpuState::VBlank
        } else {
            self.state = PpuState::OAMFetch
        }
    }
    fn vblank(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
        bus.set_lyc(bus.get_ly() + 1);
        bus.set_bit(0xFF41, 2, bus.get_ly() == bus.get_lyc());
        if bus.get_ly() == bus.get_lyc() {
            bus.set_int_request_lcd(true);
        }
        if bus.get_ly() == 153 {
            output.refresh();
            bus.set_ly(0);
            self.state = OAMFetch
        }
    }
}
