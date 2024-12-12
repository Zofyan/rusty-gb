use std::alloc::System;
use crate::bus::{Bus, OAM};
use crate::fetcher::Fetcher;
use crate::output::Output;
use crate::window_fetcher::WindowFetcher;
use bitfield::Bit;
use std::cmp::min;
use std::intrinsics::write_bytes;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use time::Time;

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
    pub fn new(index: usize, bus: &Bus) -> OAM {
        let address = OAM + index as u16 * 4;
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
    pub fn empty() -> OAM {
        OAM {
            address: 0xDF,
            y: 0xDF,
            x: 0xDF,
            tile_index: 0,
            palette: true,
            flip_x: true,
            flip_y: true,
            priority: true,
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

impl Clone for OAM {
    fn clone(&self) -> Self {
        OAM {
            address: 0,
            y: 0,
            x: 0,
            tile_index: 0,
            palette: false,
            flip_x: false,
            flip_y: false,
            priority: false,
        }
    }
}

impl Copy for OAM {

}

pub enum PpuState {
    OAMFetch,
    PixelTransfer,
    HBlank,
    VBlank,
}

pub struct Ppu {
    pub ticks: u16,
    pub state: PpuState,
    pub oambuffer: [OAM; 40],
    pub oam_selection: [bool; 40],
    x: i16,
    x_shift: u16,
    y: i16,
    y_shift: u16,
    fetcher: Fetcher,
    window_fetcher: WindowFetcher,
    ticks_pixeltransfer: u16,
    timer: u128
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            ticks: 0,
            state: PpuState::OAMFetch,
            oambuffer: [OAM::empty(); 40],
            oam_selection: [false; 40],
            x: 0,
            x_shift: 0,
            y: 0,
            y_shift: 0,
            fetcher: Fetcher::new(),
            window_fetcher: WindowFetcher::new(),
            ticks_pixeltransfer: 0,
            timer: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
        }
    }
    fn set_ppu_state(&mut self, bus: &mut Bus, state: u8) {
        let val = (bus.get(0xFF41) & 0b11111100) | state;
        bus.set(0xFF41, val);
    }
    pub fn tick(&mut self, bus: &mut Bus, output: &mut dyn Output) {
        self.ticks += 1;
        match self.state {
            PpuState::OAMFetch => {
                self.set_ppu_state(bus, 2);
                self.oam_fetch(bus, output);
            }
            PpuState::PixelTransfer => {
                self.set_ppu_state(bus, 3);
                self.pixel_tranfer(bus, output);
            }
            PpuState::HBlank => {
                self.set_ppu_state(bus, 0);
                self.hblank(bus, output);
            }
            PpuState::VBlank => {
                self.set_ppu_state(bus, 1);
                self.vblank(bus, output);
            }
        }
    }
    fn oam_fetch(&mut self, bus: &mut Bus, output: &mut dyn Output) {
        if self.ticks == 40 {
            self.state = PpuState::PixelTransfer;
            self.ticks = 0;

            let tile_line = bus.get_ly() % 8;
            let mut tileMapRowAddr = match bus.get_ldlc_bg_tilemap() {
                true => 0x9C00,
                false => 0x9800,
            };
            self.fetcher.reset(tileMapRowAddr, tile_line, bus);

            tileMapRowAddr = match bus.get_ldlc_window_tilemap() {
                true => 0x9C00,
                false => 0x9800,
            } + ((bus.get_ly() - bus.get(0xFF4A)) as u8 / 8) as u16 * 32;
            self.window_fetcher.reset(tileMapRowAddr, tile_line);

            self.x_shift = (bus.get_scx() % 8) as u16;
            self.y_shift = (bus.get_scy() % 8) as u16;

            self.x = -(self.x_shift as i16);

            self.ticks_pixeltransfer = 168;

            self.oam_selection.fill(false);
            let mut count = 0u8;
            for i in 0..40usize {
                let oam = OAM::new(i, bus);
                if (bus.get_ly().wrapping_sub(oam.y).wrapping_sub(16)) < (match bus.get_ldlc_obj_size() { true => 16, false => 8, }) {
                    self.oambuffer[i] = oam;
                    self.oambuffer[i].x.wrapping_sub(8);
                    self.oam_selection[i] = true;

                    self.ticks_pixeltransfer += (11 - min(5, (self.x.checked_add_unsigned(bus.get_scx() as u16).unwrap()) % 8 )) as u16;
                    count += 1;
                    if count >= 10 {
                        break;
                    }
                }
            }
        }
    }
    fn oam_tranfer(&mut self, bus: &mut Bus, transparent_bg: bool, output: &mut dyn Output) {
        let mut sprite_pixel = 0;
        for i in 0..40 {
            let oam = OAM::new(i, bus);
            if  (self.x.checked_sub_unsigned(oam.x as u16).unwrap()) < 8 && bus.get_ldlc_obj_enable() && self.oam_selection[i] {
                let mut offset = 0x8000;
                if (bus.get_ldlc_obj_size()) {
                    if (oam.flip_y) {offset += ((oam.tile_index | 0x01) * 16) as u16;}
                    else {offset += ((oam.tile_index & 0xFE) * 16) as u16;}
                } else {
                    offset += (oam.tile_index * 16) as u16;
                }

                let mut addr = offset;
                if (oam.flip_y) {addr += ((8 - (bus.get_ly() + 16 - oam.y)) * 2) as u16;}
                else {addr += ((bus.get_ly() + 16 - oam.y) * 2) as u16;}

                let mut bit_shift = 7 - self.x.checked_sub_unsigned(oam.x as u16).unwrap();
                if (oam.flip_x) {bit_shift = self.x.checked_sub_unsigned(oam.x as u16).unwrap();}

                let mut data = bus.get(addr);
                sprite_pixel = ((data >> bit_shift) & 0x1);
                data = bus.get(addr + 1);
                sprite_pixel |= ((data >> bit_shift) & 0x1) << 1;
                if !oam.priority || transparent_bg {
                    output.write_pixel(self.x as u16, bus.get_ly() as u16, sprite_pixel, oam.palette);
                }
                //if (oam.priority) sprite_pixel = 0;
            }
        }
    }
    fn pixel_tranfer(&mut self, bus: &mut Bus, output: &mut dyn Output) {

        if self.x == 168 {
            if self.ticks >= self.ticks_pixeltransfer {
                if bus.get_ldlc_stat_hblank_stat_int() {
                    bus.set_int_request_lcd(true);
                }
                self.state = PpuState::HBlank;
            }
            return;
        }
        self.fetcher.tick(bus);
        let mut pixel = 0;
        let mut window_pixel = 0;

        if !self.fetcher.fifo_bg.is_empty() {
            pixel = self.fetcher.fifo_bg.pop().unwrap().to_owned();
            output.write_pixel(self.x as u16, bus.get_ly() as u16, pixel, false);

            let wy = bus.get(0xFF4A);
            let wx = bus.get(0xFF4B);
            if (bus.get_ldlc_window_enable()) {
                self.window_fetcher.tick(bus);
                if (wy <= bus.get_ly() && self.x >= 7 && self.x >= wx as i16) {
                    if (!self.window_fetcher.fifo_bg.is_empty()) {
                        window_pixel = self.window_fetcher.fifo_bg.pop().unwrap().to_owned();
                        if bus.get_ldlc_bd_window_enable() && bus.get_ldlc_window_enable(){
                            println!("window - x: {}, y: {}, color: {}", self.x - 7, bus.get_ly(), window_pixel);
                            output.write_pixel((self.x - 7) as u16, bus.get_ly() as u16, window_pixel, false);
                        }
                    }
                }
            }
            bus.set_ly(bus.get_ly() - (bus.get_scy() % 8));
            self.oam_tranfer(bus, pixel == 0 && (window_pixel == 0 || bus.get_ldlc_window_enable()), output);
            //if(!pixel && !window_pixel) lcd->write_pixel(x, bus.get_ly(), oamtransfer(), 1, true);
            self.x += 1;
        }
    }
    fn hblank(&mut self, bus: &mut Bus, output: &mut dyn Output) {
        if self.ticks != 456 {
            return;
        }
        self.ticks = 0;
        bus.set_ly(bus.get_ly() + 1);

        if bus.get_ly() == bus.get_lyc() {
            bus.setb(false, false, 2, 0xFF41);
            if bus.get_ldlc_stat_lyc_ly_stat_int() {
                bus.set_int_request_lcd(true);
            }
        } else {
            bus.reset(false, false, 2, 0xFF41);
        }

        if bus.get_ly() == 144 {
            bus.set_int_request_vblank(true);
            if bus.get_ldlc_stat_vblank_stat_int() {
                bus.set_int_request_lcd(true);
            }
            self.state = PpuState::VBlank
        } else {
            for i in 0..40 {
                self.oambuffer[i] = OAM::empty()
            }
            self.state = PpuState::OAMFetch
        }
    }
    fn vblank(&mut self, bus: &mut Bus, output: &mut dyn Output) {
        if self.ticks == 456 {
            return;
        }
        self.ticks = 0;
        bus.set_ly(bus.get_ly() + 1);

        if bus.get_ly() == 153 {
            self.fetcher.tile_map = match bus.get_ldlc_bg_tilemap() { true => 0x9C00, false => 0x9800 };
            bus.set_ly(0);

            if bus.get_ldlc_stat_oam_stat_int() {
                bus.set_int_request_lcd(true);
            }

            for i in 0..40 {
                self.oambuffer[i] = OAM::empty()
            }
            self.state = PpuState::OAMFetch;
            println!("doing refresh");
            output.refresh();
            /*if self.timer + 16 > SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() {
                println!("waiting {} ms", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - self.timer);
                thread::sleep_ms((SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - self.timer) as u32);
                self.timer = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            }*/
        }

        if bus.get_ly() == bus.get_lyc() {
            bus.setb(false, false, 2, 0xFF41);
            if bus.get_ldlc_stat_lyc_ly_stat_int() {
                bus.set_int_request_lcd(true);
            }
        } else {
            bus.reset(false, false, 2, 0xFF41);
        }
    }
}
