use crate::bus::{Bus, OAM};
use crate::fetcher::Fetcher;
use crate::output::Output;
use crate::window_fetcher::WindowFetcher;
use bitfield::Bit;
use std::alloc::System;
use std::cmp::min;
use std::intrinsics::write_bytes;
use std::ops::Deref;
use std::thread;

const PPU_LINE_LENGTH: usize = 456;
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
        let address = OAM + (index as u16) * 4;
        OAM {
            address,
            y: bus._get(address),
            x: bus._get(address + 1),
            tile_index: bus._get(address + 2),
            palette: bus._get(address + 3).bit(4),
            flip_x: bus._get(address + 3).bit(5),
            flip_y: bus._get(address + 3).bit(6),
            priority: bus._get(address + 3).bit(7),
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

impl Copy for OAM {}

#[derive(Clone)]
pub enum PpuState {
    HBlank = 0,
    VBlank = 1,
    OAMFetch = 2,
    PixelTransfer = 3,
}

pub struct Ppu {
    pub ticks: usize,
    pub state: PpuState,
    pub oambuffer: [OAM; 40],
    pub oam_selection: [bool; 40],
    x: i16,
    x_shift: i16,
    y: i16,
    y_shift: i16,
    fetcher: Fetcher,
    window_fetcher: WindowFetcher,
    target_ticks: usize
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            ticks: PPU_LINE_LENGTH,
            target_ticks: PPU_LINE_LENGTH - 80,
            state: PpuState::OAMFetch,
            oambuffer: [OAM::empty(); 40],
            oam_selection: [false; 40],
            x: 0,
            x_shift: 0,
            y: 0,
            y_shift: 0,
            fetcher: Fetcher::new(),
            window_fetcher: WindowFetcher::new()
        }
    }
    fn set_ppu_state(&mut self, bus: &mut Bus, state: PpuState) {
        let val = (bus._get(0xFF41) & 0b11111100) | state.clone() as u8;
        self.state = state.clone();
        bus.set(0xFF41, val);
        self.target_ticks = match state {
            PpuState::OAMFetch => PPU_LINE_LENGTH - 80,
            PpuState::PixelTransfer => PPU_LINE_LENGTH - 80 - 172,
            PpuState::HBlank => 0,
            PpuState::VBlank => 0
        };
        self.ticks = match state {
            PpuState::OAMFetch => 456,
            PpuState::PixelTransfer => self.ticks,
            PpuState::HBlank => self.ticks,
            PpuState::VBlank => 456
        }
    }

    fn dma_tranfer(&self, bus: &mut Bus) {
        bus.set((bus.dma_address & 0xFF) + 0xFE00, bus._get(bus.dma_address));
        if bus.dma_address & 0xFF == 0x9F {
            bus.dma_address = 0;
        } else {
            bus.dma_address += 1;
        }
    }

    pub fn tick(&mut self, bus: &mut Bus, output: &mut dyn Output) {
        self.ticks -= 1;
        if bus.dma_address != 0 {
            self.dma_tranfer(bus);
        }
        match self.state {
            PpuState::OAMFetch => {
                self.oam_fetch(bus);
            }
            PpuState::PixelTransfer => {
                self.pixel_tranfer(bus, output);
            }
            PpuState::HBlank => {
                self.hblank(bus);
            }
            PpuState::VBlank => {
                self.vblank(bus, output);
            }
        }
    }
    fn oam_fetch(&mut self, bus: &mut Bus) {
        if self.ticks == self.target_ticks {
            let tile_line = bus.get_ly() % 8;

            let mut tile_map_row_addr = match bus.get_ldlc_bg_tilemap() {
                true => 0x9C00,
                false => 0x9800,
            };
            self.fetcher.reset(tile_map_row_addr, tile_line, bus);

            tile_map_row_addr = match bus.get_ldlc_window_tilemap() {
                true => 0x9C00,
                false => 0x9800,
            } + ((bus.get_ly() - bus._get(0xFF4A))/ 8) as u16 * 32;
            self.window_fetcher.reset(tile_map_row_addr, tile_line);

            self.x_shift = (bus.get_scx() % 8) as i16;
            self.y_shift = (bus.get_scy() % 8) as i16;
            self.x = -self.x_shift;

            self.oam_selection.fill(false);
            let mut count = 0u8;
            for i in 0..40 {
                let oam = OAM::new(i, bus);
                if (bus.get_ly().wrapping_sub(oam.y.wrapping_sub(16))) < (match bus.get_ldlc_obj_size() { true => 16, false => 8, })
                {
                    self.oambuffer[i] = oam;
                    self.oambuffer[i].x = self.oambuffer[i].x.wrapping_sub(8);
                    self.oam_selection[i] = true;

                    self.target_ticks -= (11 - min(5, (self.x.checked_add_unsigned(bus.get_scx() as u16).unwrap()) % 8)) as usize;

                    count += 1;
                    if count >= 10 {
                        break;
                    }
                }
            }

            self.set_ppu_state(bus, PpuState::PixelTransfer);
        }
    }
    fn oam_tranfer(&mut self, bus: &mut Bus, transparent_bg: bool, output: &mut dyn Output) {
        let mut final_pixel = (0, false, 255);
        for i in 0..40 {
            let oam = self.oambuffer[i];
            let x_ok = self.x - (oam.x as i16) < 8 && self.x - (oam.x as i16) >= 0;
            if x_ok && bus.get_ldlc_obj_enable() && self.oam_selection[i] {
                let mut offset = 0x8000;
                if bus.get_ldlc_obj_size() {
                    if oam.flip_y {
                        offset += ((oam.tile_index | 0x01) as u16) * 0x10;
                    } else {
                        offset += ((oam.tile_index & 0xFE) as u16) * 0x10;
                    }
                } else {
                    offset += (oam.tile_index as u16) * 0x10;
                }

                let mut addr = offset;
                if oam.flip_y {
                    addr += ((8 - (bus.get_ly() + 16 - oam.y)) * 2) as u16;
                } else {
                    addr += ((bus.get_ly() + 16 - oam.y) * 2) as u16;
                }

                let mut bit_shift = 7 - self.x.checked_sub_unsigned(oam.x as u16).unwrap();
                if oam.flip_x {
                    bit_shift = self.x.checked_sub_unsigned(oam.x as u16).unwrap();
                }

                let mut data = bus._get(addr);
                let mut sprite_pixel = data.overflowing_shr(bit_shift as u32).0 & 0x1;
                data = bus._get(addr + 1);
                sprite_pixel |= (data.overflowing_shr(bit_shift as u32).0 & 0x1) << 1;
                if !oam.priority || transparent_bg {
                    if sprite_pixel != 0 && oam.x < final_pixel.2 {
                        final_pixel.0 = sprite_pixel;
                        final_pixel.1 = oam.palette;
                        final_pixel.2 = oam.x;
                    }
                }
            }
            if final_pixel.2 < 255  {
                output.write_pixel(
                    self.x as u16,
                    bus.get_ly() as u16,
                    final_pixel.0,
                    final_pixel.1,
                    2
                );
            }
        }
    }
    fn pixel_tranfer(&mut self, bus: &mut Bus, output: &mut dyn Output) {

        self.fetcher.tick(bus);
        let mut pixel = 0;
        let mut window_pixel = 0;

        if !self.fetcher.fifo_bg.is_empty() {
            pixel = self.fetcher.fifo_bg.pop().unwrap().to_owned();
            output.write_pixel(self.x as u16, bus.get_ly() as u16, pixel, false, 0);

            let wy = bus._get(0xFF4A);
            let wx = bus._get(0xFF4B);
            if bus.get_ldlc_window_enable() {
                self.window_fetcher.tick(bus);
                if wy <= bus.get_ly() && self.x + 7 >= wx as i16 {
                    if !self.window_fetcher.fifo_bg.is_empty() {
                        window_pixel = self.window_fetcher.fifo_bg.pop().unwrap().to_owned();
                        if bus.get_ldlc_bd_window_enable() && bus.get_ldlc_window_enable() {
                            output.write_pixel(self.x as u16, bus.get_ly() as u16, window_pixel, false, 1);
                        }
                    }
                }
            }
            self.oam_tranfer(bus, pixel == 0 || window_pixel == 0 || bus.get_ldlc_bd_window_enable(), output);
            self.x += 1;
        } else {
            self.target_ticks -= 1;
        }


        if self.ticks == self.target_ticks + 1 {
            if bus.get_ldlc_stat_hblank_stat_int() {
                bus.set_int_request_lcd(true);
            }
            self.set_ppu_state(bus, PpuState::HBlank);
        }
    }
    fn hblank(&mut self, bus: &mut Bus) {
        if self.ticks == self.target_ticks {
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
                self.set_ppu_state(bus, PpuState::VBlank);
            } else {
                for i in 0..40 {
                    self.oambuffer[i] = OAM::empty()
                }
                self.set_ppu_state(bus, PpuState::OAMFetch);
            }
        }
    }
    fn vblank(&mut self, bus: &mut Bus, output: &mut dyn Output) {
        if self.ticks == self.target_ticks {
            if bus.get_ly() == 153 {
                bus.set_ly(0);

                if bus.get_ldlc_stat_oam_stat_int() {
                    bus.set_int_request_lcd(true);
                }

                for i in 0..40 {
                    self.oambuffer[i] = OAM::empty()
                }
                self.set_ppu_state(bus, PpuState::OAMFetch);
                output.refresh();
            } else {
                bus.set_ly(bus.get_ly() + 1);
                self.set_ppu_state(bus, PpuState::VBlank);
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
}
