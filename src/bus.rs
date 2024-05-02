#![allow(dead_code)]

use bitfield::{Bit, BitMut};

pub const ROM_0: u16 = 0x0000;
pub const ROM_0_END: u16 = 0x3FFF;
pub const ROM_N: u16 = 0x4000;
pub const ROM_N_END: u16 = 0x7FFF;
pub const VRAM: u16 = 0x8000;
pub const VRAM_END: u16 = 0x9FFF;
pub const ERAM: u16 = 0xA000;
pub const ERAM_END: u16 = 0xBFFF;
pub const WRAM_0: u16 = 0xC000;
pub const WRAM_0_END: u16 = 0xCFFF;
pub const WRAM_N: u16 = 0xD000;
pub const WRAM_N_END: u16 = 0xDFFF;
pub const OAM: u16 = 0xFE00;
pub const OAM_END: u16 = 0xFE9F;
pub const IO_REGISTERS: u16 = 0xFF00;
pub const IO_REGISTERS_END: u16 = 0xFF7F;
pub const HRAM: u16 = 0xFF80;
pub const HRAM_END: u16 = 0xFFFE;
pub const INT_ENABLE: u16 = 0xFFFF;
pub const INT_ENABLE_END: u16 = 0xFFFF;
pub const INT_REQUEST: u16 = 0xFF0F;

pub const ROM_0_SIZE: u16 = ROM_0_END - ROM_0 + 1;
pub const ROM_N_SIZE: u16 = ROM_N_END - ROM_N + 1;
pub const VRAM_SIZE: u16 = VRAM_END - VRAM + 1;
pub const ERAM_SIZE: u16 = ERAM_END - ERAM + 1;
pub const WRAM_0_SIZE: u16 = WRAM_0_END - WRAM_0 + 1;
pub const WRAM_N_SIZE: u16 = WRAM_N_END - WRAM_N + 1;
pub const OAM_SIZE: u16 = OAM_END - OAM + 1;
pub const IO_REGISTERS_SIZE: u16 = IO_REGISTERS_END - IO_REGISTERS + 1;
pub const HRAM_SIZE: u16 = HRAM_END - HRAM + 1;
pub const INT_ENABLE_SIZE: u16 = INT_ENABLE_END - INT_ENABLE + 1;

pub struct Bus {
    rom_0: [u8; ROM_0_SIZE as usize],
    rom_n: [u8; ROM_N_SIZE as usize],
    vram: [u8; VRAM_SIZE as usize],
    eram: [u8; ERAM_SIZE as usize],
    wram_0: [u8; WRAM_0_SIZE as usize],
    wram_n: [u8; WRAM_N_SIZE as usize],
    oam: [u8; OAM_SIZE as usize],
    io_registers: [u8; IO_REGISTERS_SIZE as usize],
    hram: [u8; HRAM_SIZE as usize],
    int_enable: [u8; INT_ENABLE_SIZE as usize],
}

impl Bus {
    pub fn new() -> Bus {
        Bus { rom_0: [0; ROM_0_SIZE as usize], rom_n: [0; ROM_N_SIZE as usize], vram: [0; VRAM_SIZE as usize], eram: [0; ERAM_SIZE as usize], wram_0: [0; WRAM_0_SIZE as usize], wram_n: [0; WRAM_N_SIZE as usize], oam: [0; OAM_SIZE as usize], io_registers: [0; IO_REGISTERS_SIZE as usize], hram: [0; HRAM_SIZE as usize], int_enable: [0; INT_ENABLE_SIZE as usize] }
    }
    pub fn get_target_mut(&mut self, address: u16) -> &mut u8 {
        match address {
            ..=ROM_0_END => &mut self.rom_0[(address - ROM_0) as usize],
            ROM_N..=ROM_N_END => &mut self.rom_n[(address - ROM_N) as usize],
            VRAM..=VRAM_END => &mut self.vram[(address - VRAM) as usize],
            ERAM..=ERAM_END => &mut self.eram[(address - ERAM) as usize],
            WRAM_0..=WRAM_0_END => &mut self.wram_0[(address - WRAM_0) as usize],
            WRAM_N..=WRAM_N_END => &mut self.wram_n[(address - WRAM_N) as usize],
            OAM..=OAM_END => &mut self.oam[(address - OAM) as usize],
            IO_REGISTERS..=IO_REGISTERS_END => &mut self.io_registers[(address - IO_REGISTERS) as usize],
            HRAM..=HRAM_END => &mut self.hram[(address - HRAM) as usize],
            INT_ENABLE..=INT_ENABLE_END => &mut self.int_enable[(address - INT_ENABLE) as usize],
            _ => panic!("Not implemented yet!")
        }
    }
    pub fn get_target(&self, address: u16) -> &u8 {
        match address {
            ..=ROM_0_END => &self.rom_0[(address - ROM_0) as usize],
            ROM_N..=ROM_N_END => &self.rom_n[(address - ROM_N) as usize],
            VRAM..=VRAM_END => &self.vram[(address - VRAM) as usize],
            ERAM..=ERAM_END => &self.eram[(address - ERAM) as usize],
            WRAM_0..=WRAM_0_END => &self.wram_0[(address - WRAM_0) as usize],
            WRAM_N..=WRAM_N_END => &self.wram_n[(address - WRAM_N) as usize],
            OAM..=OAM_END => &self.oam[(address - OAM) as usize],
            IO_REGISTERS..=IO_REGISTERS_END => &self.io_registers[(address - IO_REGISTERS) as usize],
            HRAM..=HRAM_END => &self.hram[(address - HRAM) as usize],
            INT_ENABLE..=INT_ENABLE_END => &self.int_enable[(address - INT_ENABLE) as usize],
            _ => panic!("Not implemented yet!")
        }
    }
    pub fn get(&self, address: u16) -> u8 {
        *self.get_target(address)
    }
    pub fn gets(&self, address: u16) -> i8 {
        *self.get_target(address) as i8
    }
    pub fn get16(&self, address: u16) -> u16 {
        let (target1, target2) = (self.get_target(address), self.get_target(address + 1));
        let v1 = *target1 as u16;
        let v2 = *target2 as u16;
        v2 << 8 | v1
    }
    pub fn set(&mut self, address: u16, value: u8) {
        let target = self.get_target_mut(address);
        *target = value
    }
    pub fn set16(&mut self, address: u16, value: u16) {
        let target = self.get_target_mut(address);
        *target = value as u8;
        let target = self.get_target_mut(address + 1);
        *target = (value >> 8) as u8
    }
    
    pub fn rl(&mut self, _: bool, carry_value: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        value.set_bit(0, carry_value);
        value = value.rotate_left(1);
        self.set(address, value);
        (value == 0, false, false, value.bit(0))
    }
    pub fn rlc(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        value = value.rotate_left(1);
        self.set(address, value);
        (value == 0, false, false, value.bit(0))
    }
    pub fn rr(&mut self, _: bool, carry_value: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        value.set_bit(7, carry_value);
        value = value.rotate_right(1);
        self.set(address, value);
        (value == 0, false, false, value.bit(7))
    }
    pub fn rrc(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        value = value.rotate_right(1);
        self.set(address, value);
        (value == 0, false, false, value.bit(7))
    }
    pub fn sla(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c= value.bit(7);
        value <<= 1;
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn srl(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c= value.bit(0);
        value >>= 1;
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn sra(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c= value.bit(0);
        value = (value >> 1) | value;
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn swap(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        value = (value << 4) | (value >> 4);
        self.set(address, value);
        (value == 0, false, false, false)
    }
    pub fn bit(&mut self, _: bool, _: bool, bit: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        (!value.bit(bit), false, true, false)
    }
    pub fn reset(&mut self, _: bool, _: bool, bit: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        value.set_bit(bit, false);
        self.set(address, value);
        (false, false, false, false)
    }
    pub fn setb(&mut self, _: bool, _: bool, bit: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        value.set_bit(bit, true);
        self.set(address, value);
        (false, false, false, false)
    }

    pub fn load_rom(&mut self, buffer: Vec<u8>) {
        self.rom_0[..ROM_0_SIZE as usize].copy_from_slice(&buffer[..=ROM_0_END as usize]);
        self.rom_n[..ROM_N_SIZE as usize].copy_from_slice(&buffer[ROM_N as usize..=ROM_N_END as usize]);
    }
}
