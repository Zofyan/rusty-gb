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
    data0: u8,
    data1: u8,
}

impl OAM {
    pub fn new(index: usize, bus: &Bus) -> OAM {
        let address = OAM + (index as u16) * 4;
        let mut oam = OAM::empty();
        oam.y = bus._get(address);
        oam.address = address;
        oam
    }
    pub fn init(&mut self, bus: &Bus) {
        self.x = bus._get(self.address + 1);
        self.tile_index = bus._get(self.address + 2);
        let tmp = bus._get(self.address + 3);
        self.palette = tmp.bit(4);
        self.flip_x = tmp.bit(5);
        self.flip_y = tmp.bit(6);
        self.priority = tmp.bit(7);
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
            data0: 0,
            data1: 0,
        }
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
            data0: 0,
            data1: 0,
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
    pub oambuffer: Vec<OAM>,
    x: i16,
    x_shift: i16,
    y: i16,
    y_shift: i16,
    window_y_hit: bool,
    fetcher: Fetcher,
    window_fetcher: WindowFetcher,
    target_ticks: usize,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            ticks: PPU_LINE_LENGTH,
            target_ticks: PPU_LINE_LENGTH - 80,
            state: PpuState::OAMFetch,
            oambuffer: Vec::with_capacity(10),
            x: 0,
            x_shift: 0,
            y: 0,
            y_shift: 0,
            window_y_hit: false,
            fetcher: Fetcher::new(),
            window_fetcher: WindowFetcher::new(),
        }
    }
    fn set_ppu_state(&mut self, bus: &mut Bus, state: PpuState) {
        bus.ppu_state = state.clone();
        let val = (bus.registers.lcds & 0b11111100) | state.clone() as u8;
        self.state = state.clone();
        bus.set(0xFF41, val);
        self.target_ticks = match state {
            PpuState::OAMFetch => PPU_LINE_LENGTH - 80,
            PpuState::PixelTransfer => self.target_ticks - 172,
            PpuState::HBlank => 0,
            PpuState::VBlank => 0,
        };
        self.ticks = match state {
            PpuState::OAMFetch => 456,
            PpuState::PixelTransfer => self.ticks,
            PpuState::HBlank => self.ticks,
            PpuState::VBlank => 456,
        }
    }

    fn dma_tranfer(&self, bus: &mut Bus) {
        bus._set((bus.dma_address & 0xFF) + 0xFE00, bus._get(bus.dma_address));
        if bus.dma_address & 0xFF == 0x9F {
            bus.dma_address = 0;
        } else {
            bus.dma_address += 1;
        }
    }

    pub fn tick(&mut self, bus: &mut Bus, output: &mut Box<dyn Output>, mut ticks: usize) {
        while ticks > 0 {
            if bus.dma_address != 0 {
                for _ in 0..4 {
                    self.dma_tranfer(bus);
                }
            }
            let consumed_ticks = match self.state {
                PpuState::OAMFetch => {
                    self.oam_fetch(bus, ticks)
                }
                PpuState::PixelTransfer => {
                    self.pixel_tranfer(bus, output, ticks)
                }
                PpuState::HBlank => {
                    self.hblank(bus, ticks)
                }
                PpuState::VBlank => {
                    self.vblank(bus, output, ticks)
                }
            };
            ticks -= consumed_ticks;
        }
    }

    fn oam_fetch(&mut self, bus: &mut Bus, ticks: usize) -> usize {
        let mut i = 0;
        while i < ticks {
            self.ticks -= 4;
            i += 1;
            if self.ticks <= self.target_ticks {
                break;
            }
        }
        if i == ticks && self.ticks > self.target_ticks {
            return ticks;
        }
        self.window_y_hit |= bus.get_wy() == bus.get_ly();
        let tile_line = bus.get_ly() % 8;

        let mut tile_map_row_addr = match bus.get_ldlc_bg_tilemap() {
            true => 0x9C00,
            false => 0x9800,
        };
        self.fetcher.reset(tile_map_row_addr, tile_line, bus);

        tile_map_row_addr = match bus.get_ldlc_window_tilemap() {
            true => 0x9C00,
            false => 0x9800,
        } + ((bus.get_ly() - bus._get(0xFF4A)) / 8) as u16 * 32;
        self.window_fetcher.reset(tile_map_row_addr, tile_line, bus);

        self.x_shift = (bus.get_scx() % 8) as i16;
        self.y_shift = (bus.get_scy() % 8) as i16;
        self.x = -self.x_shift;

        self.oambuffer.clear();
        let mut count = 0u8;
        for i in 0..40 {
            let mut oam = OAM::new(i, bus);
            if (bus.get_ly().wrapping_sub(oam.y.wrapping_sub(16)))
                < (match bus.get_ldlc_obj_size() {
                    true => 16,
                    false => 8,
                })
            {
                oam.init(bus);
                //oam.x = oam.x.saturating_sub(8);

                let mut offset = 0x8000;
                let obj_size = bus.get_ldlc_obj_size();
                if obj_size {
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

                oam.data0 = bus._get(addr);
                oam.data1 = bus._get(addr + 1);

                self.oambuffer.push(oam);

                self.target_ticks -= (11
                    - min(
                        5,
                        (self.x.checked_add_unsigned(bus.get_scx() as u16).unwrap()) % 8,
                    )) as usize;

                count += 1;
                if count >= 10 {
                    break;
                }
            }
        }

        self.oambuffer.sort_by_key(|oam| oam.x);

        self.set_ppu_state(bus, PpuState::PixelTransfer);
        i
    }

    fn oam_tranfer(&mut self, bus: &mut Bus, transparent_bg: bool, mut output: &mut Box<dyn Output>) -> bool {
        if !bus.get_ldlc_obj_enable() {
            return false;
        }
        let mut final_pixel = (0, false);
        for oam in self.oambuffer.iter().rev() {
            if self.x + 8 - (oam.x as i16) < 8 && self.x + 8 - (oam.x as i16) >= 0 {
                let mut bit_shift = 7 - self.x.saturating_sub_unsigned(oam.x as u16);
                if oam.flip_x {
                    bit_shift = self.x.wrapping_sub_unsigned(oam.x as u16);
                }

                let mut sprite_pixel = oam.data0.overflowing_shr(bit_shift as u32).0 & 0x1;
                sprite_pixel |= (oam.data1.overflowing_shr(bit_shift as u32).0 & 0x1) << 1;
                if !oam.priority || transparent_bg {
                    if sprite_pixel != 0 {
                        final_pixel.0 = sprite_pixel;
                        final_pixel.1 = oam.palette;
                        output.write_pixel(
                            self.x as u16,
                            bus.get_ly() as u16,
                            final_pixel.0,
                            final_pixel.1,
                            2,
                        );
                        return true
                    }
                }
            }

        }
        false
    }

    fn pixel_tranfer(&mut self, bus: &mut Bus, mut output: &mut Box<dyn Output>, ticks: usize) -> usize {
        let mut pixel = 255;
        let mut debug = 0;
        let condition = self.window_y_hit && bus.get_ldlc_window_enable() && self.x + 7 >= bus.get_wx() as i16;

        let mut i = 0;
        while i < ticks {
            self.ticks -= 4;
            i += 1;

            if condition {
                self.window_fetcher.tick(bus);
                debug = 1;

                while !self.window_fetcher.fifo_bg.is_empty() {
                    pixel = self.window_fetcher.fifo_bg.pop().unwrap().to_owned();
                    let transparent_bg = pixel == 0;
                    if !self.oam_tranfer(bus, transparent_bg, output) {
                        output.write_pixel(self.x as u16, bus.get_ly() as u16, pixel, false, debug);
                    }
                    self.x += 1;
                }
            } else {
                self.fetcher.tick(bus);
                while !self.fetcher.fifo_bg.is_empty() {
                    pixel = self.fetcher.fifo_bg.pop().unwrap().to_owned();
                    let transparent_bg = pixel == 0;
                    if !self.oam_tranfer(bus, transparent_bg, output){
                        output.write_pixel(self.x as u16, bus.get_ly() as u16, pixel, false, debug);
                    }
                    self.x += 1;
                }
            }
            if pixel != 255 { self.target_ticks = self.target_ticks.saturating_sub(4) }
            if self.ticks <= self.target_ticks + 1 {
                break;
            }
        }
        if i == ticks && self.ticks > self.target_ticks {
            return ticks;
        }

        if self.ticks <= self.target_ticks + 1 {
            if bus.get_ldlc_stat_hblank_stat_int() {
                bus.set_int_request_lcd(true);
            }
            self.set_ppu_state(bus, PpuState::HBlank);
        }
        i
    }

    fn hblank(&mut self, bus: &mut Bus, ticks: usize) ->usize{
        let mut i = 0;
        while i < ticks {
            self.ticks = self.ticks.saturating_sub(4);
            i += 1;
            if self.ticks <= self.target_ticks {
                break;
            }
        }
        if i == ticks && self.ticks > self.target_ticks {
            return ticks;
        }
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
            self.set_ppu_state(bus, PpuState::OAMFetch);
        }
        i
    }

    fn vblank(&mut self, bus: &mut Bus, mut output: &mut Box<dyn Output>, ticks: usize) -> usize {
        let mut i = 0;
        while i < ticks {
            self.ticks = self.ticks.saturating_sub(4);
            i += 1;
            if self.ticks <= self.target_ticks {
                break;
            }
        }
        if i == ticks && self.ticks > self.target_ticks {
            return ticks;
        }
        if bus.get_ly() == 153 {
            self.window_y_hit = false;

            bus.set_ly(0);

            if bus.get_ldlc_stat_oam_stat_int() {
                bus.set_int_request_lcd(true);
            }

            self.set_ppu_state(bus, PpuState::OAMFetch);
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
        i
    }
}
