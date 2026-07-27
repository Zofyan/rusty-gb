#![allow(dead_code)]

use std::cmp::PartialEq;
use std::io::{Read};
use bitfield::{Bit, BitMut};
use bytesize::{kib, mib};
use cloneable_file::CloneableFile;
use rand::{Rng};
use crate::input::Input;
use crate::mbc::Mbc;
use crate::memory::Memory;
use crate::output::Output;
use crate::ppu::{PpuState, OAM};
use crate::ppu::PpuState::{OAMFetch, PixelTransfer};
use crate::rom::Rom;

pub const ROM_0: usize = 0x0000;
pub const ROM_0_END: usize = 0x3FFF;
pub const ROM_N: usize = 0x4000;
pub const ROM_N_END: usize = 0x7FFF;
pub const VRAM: usize = 0x8000;
pub const VRAM_END: usize = 0x9FFF;
pub const ERAM: usize = 0xA000;
pub const ERAM_END: usize = 0xBFFF;
pub const WRAM_0: usize = 0xC000;
pub const WRAM_0_END: usize = 0xCFFF;
pub const WRAM_N: usize = 0xD000;
pub const WRAM_N_END: usize = 0xDFFF;
pub const OAM: usize = 0xFE00;
pub const OAM_END: usize = 0xFE9F;
pub const IO_REGISTERS: usize = 0xFF00;
pub const IO_REGISTERS_END: usize = 0xFF7F;
pub const HRAM: usize = 0xFF80;
pub const HRAM_END: usize = 0xFFFE;
pub const INT_ENABLE: usize = 0xFFFF;
pub const INT_ENABLE_END: usize = 0xFFFF;
pub const INT_REQUEST: usize = 0xFF0F;

pub const ROM_0_SIZE: usize = ROM_0_END - ROM_0 + 1;
pub const ROM_N_SIZE: usize = ROM_N_END - ROM_N + 1;
pub const VRAM_SIZE: usize = VRAM_END - VRAM + 1;
pub const ERAM_SIZE: usize = ERAM_END - ERAM + 1;
pub const WRAM_0_SIZE: usize = WRAM_0_END - WRAM_0 + 1;
pub const WRAM_N_SIZE: usize = WRAM_N_END - WRAM_N + 1;
pub const OAM_SIZE: usize = OAM_END - OAM + 1;
pub const IO_REGISTERS_SIZE: usize = IO_REGISTERS_END - IO_REGISTERS + 1;
pub const HRAM_SIZE: usize = HRAM_END - HRAM + 1;
pub const INT_ENABLE_SIZE: usize = INT_ENABLE_END - INT_ENABLE + 1;

pub struct MMAPRegisters {
    pub sb: u8,
    pub sc: u8,
    pub div: u8,
    pub tima: u8,
    pub tma: u8,
    pub tca: u8,
    ly: u8,
    joypad: u8,
    scy: u8,
    scx: u8,
    wx: u8,
    wy: u8,
    lcdc: u8,
    pub(crate) lcds: u8,
    bg_palette_data: u8,
    obj_palette_0: u8,
    obj_palette_1: u8,
    interrupt_enable: u8,
    interrupt_flag: u8,
}
pub struct Bus {
    pub(crate) memory: Memory,
    pub(crate) registers: MMAPRegisters,
    mbc: Mbc,
    pub(crate) rom: Rom,
    pub ppu_state: PpuState,
    pub dma_address: u16,
    pub oams: [OAM; 40]

}

impl Bus {
    pub fn new(rom: Rom) -> Bus {
        Bus {
            memory: Memory::new(),
            registers: MMAPRegisters {
                sb: 0,
                sc: 0,
                div: 0,
                tima: 0,
                tma: 0,
                tca: 0,
                ly: 91,
                joypad: 0,
                scy: 0,
                scx: 0,
                wx: 0,
                wy: 0,
                lcdc: 0,
                lcds: 0,
                bg_palette_data: 0,
                obj_palette_0: 0,
                obj_palette_1: 0,
                interrupt_enable: 0,
                interrupt_flag: 0,
            },
            mbc: Mbc::Dummy,
            rom,
            ppu_state: OAMFetch,
            dma_address: 0,
            oams: [OAM::empty(); 40],
        }
    }
    #[inline]
    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    pub fn get(&self, address: u16) -> u8 {
        match address {
            // Cartridge ROM does not go through the mapper: every MBC returned
            // `memory.get(address)` here, so the dispatch only ever cost an
            // indirect call. This is the opcode-fetch path, so it matters most.
            ..=0x7FFF => self.rom.read(address),
            0xA000..=0xBFFF => { self.mbc.read(address, &self.memory) },
            0xe000..=0xfdff | 0xfea0..=0xfeff => 0xFF,
            0xFF00 => self.registers.joypad,
            0xFF01 => self.registers.sb,
            0xFF02 => self.registers.sc,
            0xFF04 => self.registers.div,
            0xFF05 => self.registers.tima,
            0xFF06 => self.registers.tma,
            0xFF07 => self.registers.tca,
            0xFF40 => self.registers.lcdc,
            0xFF41 => self.registers.lcds,
            0xFF42 => self.registers.scy,
            0xFF43 => self.registers.scx,
            0xFF44 => self.registers.ly,
            0xFF47 => self.registers.bg_palette_data,
            0xFF48 => self.registers.obj_palette_0,
            0xFF49 => self.registers.obj_palette_1,
            0xFF4A => self.registers.wy,
            0xFF4B => self.registers.wx,
            0xFF0F => self.registers.interrupt_flag,
            0xFFFF => self.registers.interrupt_enable,
            0xFF00 => {
                if self.get_joypad_dpad_buttons() && self.get_joypad_select_buttons() {
                    0xFF
                } else {
                    self.memory.get(address)
                }
            },
            _ => self.memory.get(address)
        }
    }
    pub fn _get(&self, address: u16) -> u8 {
        match address {
            0xFF00 | 0xFF40 => panic!("no"),
            0xFF41 => panic!("no"),
            0xFF42 | 0xFF43  => panic!("no"),
            0xFF44 | 0xFF47 | 0xFF48 | 0xFF49 | 0xFF0F | 0xFFFF => panic!("no"),
            _ => self.memory.get(address)
        }
    }

    pub fn inc(&mut self, address: u16) {
        self.memory.set(address, self.memory.get(address).wrapping_add(1));
    }
    pub fn gets(&self, address: u16) -> i8 {
        self.get(address) as i8
    }
    pub fn get16(&self, address: u16) -> u16 {
        let v1 = self.get(address) as u16;
        let v2 = self.get(address + 1) as u16;
        v2 << 8 | v1
    }
    #[inline]
    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    pub fn set(&mut self, address: u16, value: u8) {
        match address {
            ..=0x7FFF | 0xA000..=0xBFFF => { self.mbc.write(address, value, &mut self.memory, &mut self.rom); },
            0xe000..=0xfdff | 0xfea0..=0xfeff => {},
            0x8000..=0x9fff => {
                match self.ppu_state {
                    PixelTransfer => self.memory.set(address, value),
                    _ => self.memory.set(address, value)
                }
            },
            0xFE00..=0xFE9F => {
                match self.ppu_state {
                    PixelTransfer | OAMFetch => {},
                    _ => {
                        let index = (address as usize - OAM) / 4;
                        self.oams[index].set(value, (address & 0b11) as u8);
                        self.memory.set(address, value)
                    }
                }
            },
            0xFF41 => {
                self.registers.lcds = (value & 0b11111000) | (self.memory.get(address) & 0b111);
            },
            0xFF00 => self.registers.joypad = value,
            0xFF01 => self.registers.sb = value,
            0xFF02 => self.registers.sc = value,
            0xFF04 => self.registers.div = value,
            0xFF05 => self.registers.tima = value,
            0xFF06 => self.registers.tma = value,
            0xFF07 => self.registers.tca = value,
            0xFF40 => self.registers.lcdc = value,
            0xFF42 => self.registers.scy = value,
            0xFF43 => self.registers.scx = value,
            0xFF47 => self.registers.bg_palette_data = value,
            0xFF48 => self.registers.obj_palette_0 = value,
            0xFF49 => self.registers.obj_palette_1 = value,
            0xFF4A => self.registers.wy = value,
            0xFF4B => self.registers.wx = value,
            0xFF0F => self.registers.interrupt_flag = value,
            0xFFFF => self.registers.interrupt_enable = value,
            0xFFFF => self.registers.interrupt_enable = value,
            0xFF44 => { },
            0xFF04 => self.memory.set(address, 0),
            0xFF46 => {
                self.dma_address = value as u16 * 0x100;
            }
            _ => self.memory.set(address, value)
        }
    }
    pub fn _set(&mut self, address: u16, value: u8) {
        match address {
            0xFF00 | 0xFF40 | 0xFF41 | 0xFF42 | 0xFF43 | 0xFF44 | 0xFF47 | 0xFF48 | 0xFF49 | 0xFF0F | 0xFFFF => panic!("no"),
            _ => self.memory.set(address, value)
        }
    }
    pub fn set16(&mut self, address: u16, value: u16) {
        self.set(address, value as u8);
        self.set(address + 1, (value >> 8) as u8);
    }

    pub fn rl(&mut self, _: bool, carry_value: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c = value.bit(7);
        value <<= 1;
        value.set_bit(0, carry_value);
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn rlc(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c = value.bit(7);
        value <<= 1;
        value.set_bit(0, c);
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn rr(&mut self, _: bool, carry_value: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c = value.bit(0);
        value >>= 1;
        value.set_bit(7, carry_value);
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn rrc(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c = value.bit(0);
        value >>= 1;
        value.set_bit(7, c);
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn sla(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c = value.bit(7);
        value <<= 1;
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn srl(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c = value.bit(0);
        value >>= 1;
        self.set(address, value);
        (value == 0, false, false, c)
    }
    pub fn sra(&mut self, _: bool, _: bool, _: usize, address: u16) -> (bool, bool, bool, bool) {
        let mut value = self.get(address);
        let c = value.bit(0);
        let bit7= value.bit(7);
        value >>= 1;
        value.set_bit(7, bit7);
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
        let value = self.get(address);
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
    /// Enabled-and-requested interrupts as a bitmask, lowest bit = highest priority.
    #[inline]
    pub fn pending_interrupts(&self) -> u8 {
        self.registers.interrupt_enable & self.registers.interrupt_flag & 0x1F
    }
    #[inline]
    pub fn clear_int_request(&mut self, bit: u8) {
        self.registers.interrupt_flag &= !(1u8 << bit);
    }
    pub fn set_int_enable_joypad(&mut self, value: bool){
        self.registers.interrupt_enable.set_bit(4, value)
    }
    pub fn set_int_enable_serial(&mut self, value: bool){
        self.registers.interrupt_enable.set_bit(3, value)
    }
    pub fn set_int_enable_timer(&mut self, value: bool){
        self.registers.interrupt_enable.set_bit(2, value)
    }
    pub fn set_int_enable_lcd(&mut self, value: bool){
        self.registers.interrupt_enable.set_bit(1, value)
    }
    pub fn set_int_enable_vblank(&mut self, value: bool){
        self.registers.interrupt_enable.set_bit(0, value)
    }
    pub fn get_int_enable_joypad(&self) -> bool{
        self.registers.interrupt_enable.bit(4)
    }
    pub fn get_int_enable_serial(&self) -> bool{
        self.registers.interrupt_enable.bit(3)
    }
    pub fn get_int_enable_timer(&self) -> bool{
        self.registers.interrupt_enable.bit(2)
    }
    pub fn get_int_enable_lcd(&self) -> bool{
        self.registers.interrupt_enable.bit(1)
    }
    pub fn get_int_enable_vblank(&self) -> bool{
        self.registers.interrupt_enable.bit(0)
    }
    pub fn set_int_request_joypad(&mut self, value: bool){
        self.registers.interrupt_flag.set_bit(4, value)
    }
    pub fn set_int_request_serial(&mut self, value: bool){
        self.registers.interrupt_flag.set_bit(3, value)
    }
    pub fn set_int_request_timer(&mut self, value: bool){
        self.registers.interrupt_flag.set_bit(2, value)
    }
    pub fn set_int_request_lcd(&mut self, value: bool){
        self.registers.interrupt_flag.set_bit(1, value)
    }
    pub fn set_int_request_vblank(&mut self, value: bool){
        self.registers.interrupt_flag.set_bit(0, value)
    }
    pub fn get_int_request_joypad(&self) -> bool{
        self.registers.interrupt_flag.bit(4)
    }
    pub fn get_int_request_serial(&self) -> bool{
        self.registers.interrupt_flag.bit(3)
    }
    pub fn get_int_request_timer(&self) -> bool{
        self.registers.interrupt_flag.bit(2)
    }
    pub fn get_int_request_lcd(&self) -> bool{
        self.registers.interrupt_flag.bit(1)
    }
    pub fn get_int_request_vblank(&self) -> bool{
        self.registers.interrupt_flag.bit(0)
    }
    pub fn set_joypad_input(&mut self, bit: usize, value: bool){
        let mut val= self._get(0xFF00);
        val.set_bit(bit, !value);
        self._set(0xFF00, val)
    }
    pub fn get_ldlc_bd_window_enable(&self) -> bool {
        self.registers.lcdc.bit(0)
    }
    pub fn get_ldlc_obj_enable(&self) -> bool {
        self.registers.lcdc.bit(1)
    }
    pub fn get_ldlc_obj_size(&self) -> bool {
        self.registers.lcdc.bit(2)
    }
    pub fn get_ldlc_bg_tilemap(&self) -> bool {
        self.registers.lcdc.bit(3)
    }
    pub fn get_ldlc_bg_window_tiles(&self) -> bool {
        self.registers.lcdc.bit(4)
    }
    pub fn get_ldlc_window_enable(&self) -> bool {
        self.registers.lcdc.bit(5)
    }
    pub fn get_ldlc_window_tilemap(&self) -> bool {
        self.registers.lcdc.bit(6)
    }
    pub fn get_ldlc_lcd_ppu_enable(&self) -> bool {
        self.registers.lcdc.bit(7)
    }
    pub fn get_ldlc_stat_mode(&self) -> u8 {
        self.registers.lcds & 0b11
    }
    pub fn get_ldlc_stat_lyc_ly_flag(&self) -> bool {
        self.registers.lcds.bit(2)
    }
    pub fn get_ldlc_stat_hblank_stat_int(&self) -> bool {
        self.registers.lcds.bit(3)
    }
    pub fn get_ldlc_stat_vblank_stat_int(&self) -> bool {
        self.registers.lcds.bit(4)
    }
    pub fn get_ldlc_stat_oam_stat_int(&self) -> bool {
        self.registers.lcds.bit(5)
    }
    pub fn get_ldlc_stat_lyc_ly_stat_int(&self) -> bool {
        self.registers.lcds.bit(6)
    }

    pub fn get_scy(&self) -> u8 {
        self.registers.scy
    }
    pub fn get_scx(&self) -> u8 {
        self.registers.scx
    }
    pub fn get_ly(&self) -> u8 {
        self.registers.ly
    }
    pub fn get_lyc(&self) -> u8 {
        self._get(0xFF45)
    }
    pub fn set_scy(&mut self, value: u8) {
        self.registers.scy = value
    }
    pub fn set_scx(&mut self, value: u8) {
        self.registers.scx = value
    }
    pub fn set_ly(&mut self, value: u8) {
        self.registers.ly = value
    }
    pub fn set_lyc(&mut self, value: u8) {
        self._set(0xFF45, value)
    }
    pub fn get_wy(&self) -> u8 {
        self.registers.wy
    }
    pub fn get_wx(&self) -> u8 {
        self.registers.wx
    }
    pub fn get_joypad_select_buttons(&self) -> bool {
        self.registers.joypad.bit(5)
    }
    pub fn get_joypad_dpad_buttons(&self) -> bool {
        self.registers.joypad.bit(4)
    }
    pub fn set_joypad_set_start_down(&mut self) {
        self.registers.joypad.set_bit(3, false)
    }
    pub fn set_joypad_set_select_up(&mut self) {
        self.registers.joypad.set_bit(2, false)
    }
    pub fn set_joypad_set_b_left(&mut self) {
        self.registers.joypad.set_bit(1, false)
    }
    pub fn set_joypad_set_a_right(&mut self) {
        self.registers.joypad.set_bit(0, false)
    }
    pub fn reset_joypad_buttons(&mut self) {
        self.registers.joypad = self.registers.joypad | 0x0F;
    }
    /// Reads the cartridge header and installs the matching mapper.
    pub fn load_rom(&mut self) {
        let rom_size = match self.get(0x0148) {
            0x00 => kib(32u64),
            0x01 => kib(64u64),
            0x02 => kib(128u64),
            0x03 => kib(256u64),
            0x04 => kib(512u64),
            0x05 => mib(1u64),
            _ => panic!("Not implemented yet! {}", self.get(0x0149))
        };

        match self.get(0x0149) {
            0x00 => {},
            0x02 => {
                self.memory.eram.resize(1 * ERAM_SIZE, 0);
                self.memory.current_eram = 0;
            },
            0x03 => {
                self.memory.eram.resize(4 * ERAM_SIZE, 0);
                self.memory.current_eram = 0;
            },
            _ => panic!("Not implemented yet! {}", self.get(0x0149))
        }

        self.mbc = match self.get(0x0147) {
            0x00 => Mbc::Mbc0,
            0x01 | 0x02 | 0x03 => Mbc::Mbc1 { banking_mode: false, rom_size: rom_size as usize },
            0x05 | 0x06 => Mbc::Mbc2,
            0x0F | 0x10 | 0x11 | 0x12 | 0x13 => Mbc::Mbc3 { rtc_registers: false, rtc_register: 0x08 },
            other => panic!("MBC not implemented yet! {:#02x}", other),
        };

        self.memory.set(0xFF40, 0x91);
        self.memory.set(0xFF00, 0x00);
    }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use crate::bus::{Bus};

    #[test]
    fn rlc() {
        let mut bus = Bus::new(crate::rom::Rom::test());
        bus.set(0x8000, 0x80);

        let (z, _, h, c) = bus.rlc(false, false, 0, 0x8000);
        assert_eq!(z, false);
        assert_eq!(h, false);
        assert_eq!(c, true);
        assert_eq!(bus.get(0x8000), 0x01);
    }
    #[test]
    fn sra() {
        let mut bus = Bus::new(crate::rom::Rom::test());
        bus.set(0x8000, 0x01);

        let (z, _, h, c) = bus.sra(false, false, 0, 0x8000);
        assert_eq!(z, true);
        assert_eq!(h, false);
        assert_eq!(c, true);
        assert_eq!(bus.get(0x8000), 0x00);
    }
    #[test]
    fn rr() {
        let mut bus = Bus::new(crate::rom::Rom::test());

        bus.set(0x8000, 0x7C);

        let (_, _, _, c) = bus.rr(false, true, 0, 0x8000);
        assert_eq!(c, false);
        assert_eq!(bus.get(0x8000), 0xBE);

        bus.set(0x8000, 0x3D);

        let (_, _, _, c) = bus.rr(false, true, 0, 0x8000);
        assert_eq!(c, true);
        assert_eq!(bus.get(0x8000), 0x9E);

        bus.set(0x8000, 0xFF);

        let (_, _, _, c) = bus.rr(false, true, 0, 0x8000);
        assert_eq!(c, true);
        assert_eq!(bus.get(0x8000), 0xFF);

        bus.set(0x8000, 0x47);

        let (_, _, _, c) = bus.rr(false, false, 0, 0x8000);
        assert_eq!(c, true);
        assert_eq!(bus.get(0x8000), 0x23);

    }

    #[test]
    fn standard() {
        let mut bus = Bus::new(crate::rom::Rom::test());
        assert_eq!(bus.get_ly(), 91);
    }
}