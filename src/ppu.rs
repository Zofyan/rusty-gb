use bitfield::Bit;
use crate::bus::{Bus, OAM};
use crate::fetcher::Fetcher;
use crate::output::Output;

const LY: u16 = 0xFF44;
const LYC: u16 = 0xFF45;

pub struct OAM {
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

pub enum PpuState {
    OAMFetch,
    PixelTransfer,
    HBlank,
    VBlank,
}

pub(crate) struct Ppu {
    pub(crate) ticks: u16,
    pub(crate) state: PpuState,
    pub(crate) oambuffer: Vec<OAM>,
}

impl Ppu {
    fn set_ppu_state(&mut self, bus: &mut Bus, state: u8) {
        let val = (bus.get(0xFF41) & 0b11111100) | state;
        bus.set(0xFF41, val);
    }
    pub fn tick(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
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
        if self.ticks % 2 == 0 { return; }
        let oam = OAM::new(0xFE00 + self.ticks, &bus);
        if oam.x > 0 && bus.get_ly() + 16 >= oam.y && (bus.get_ly() < oam.y + 8) && self.oambuffer.len() < 10 { //TODO: add check for sprite mode
            self.oambuffer.push(oam);
        }
        if self.ticks == 79 {
            self.ticks = 0;
            self.state = PpuState::PixelTransfer;
            fetcher.start(0);
        }
    }
    fn pixel_tranfer(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
        if self.ticks < (160 + bus.get_scx() % 8) as u16 {
            fetcher.tick(bus);
            let empty = bus.fifo.is_empty();
            if !empty {
                let pixel = *bus.fifo.first().unwrap();
                bus.fifo.pop();
                output.write_pixel(self.ticks, bus.get(LY) as u16, pixel);
            }
        } else{
            self.ticks = 0;
            self.state = PpuState::HBlank;
        }
    }
    fn hblank(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
        if self.ticks == 455 - 80 - (160 + bus.get_scx() % 8) as u16 {
            bus.set_ly(bus.get_ly() + 1);
            if bus.get_ly() == bus.get_lyc() {
                bus.setb(false, false, 2, 0xFF41);
                bus.set_int_request_lcd(true);
            } else {
                bus.reset(false, false, 2, 0xFF41);
            }
            self.ticks = 0;
            if bus.get_ly() == 144 {
                //bus.set_int_request_vblank(true);
                self.state = PpuState::VBlank
            } else {
                self.state = PpuState::OAMFetch
            }
        }
    }
    fn vblank(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &mut Box<dyn Output>) {
        if self.ticks + 1 % 456 == 0{
            bus.set_ly(bus.get_ly() + 1);
        }
        bus.set_bit(0xFF41, 2, bus.get_ly() == bus.get_lyc());
        if bus.get_ly() == bus.get_lyc() {
            bus.set_int_request_lcd(true);
        }
        if self.ticks == 456 * 10 - 1{
            self.ticks = 0;
            output.refresh();
            bus.set_ly(0);
            self.state = PpuState::OAMFetch
        }
    }
}
