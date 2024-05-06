#![allow(dead_code)]

use bitfield::{Bit, BitMut};
use crate::memory::Memory;

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
    memory: Memory,
    pub(crate) fifo: Vec<u8>
}

impl Bus {
    pub fn new() -> Bus {
        Bus { memory: Memory::new(), fifo: vec![] }
    }
    pub fn get(&self, address: u16) -> u8 {
        match address {
            0xe000..=0xfdff | 0xfea0..=0xfeff => panic!("Nintendo says no!!!"),
            _ => self.memory.get(address)
        }
    }
    pub fn gets(&self, address: u16) -> i8 {
        self.get(address) as i8
    }
    pub fn get16(&self, address: u16) -> u16 {
        let v1 = self.get(address) as u16;
        let v2 = self.get(address + 1) as u16;
        v2 << 8 | v1
    }
    pub fn set(&mut self, address: u16, value: u8) {
        match address {
            ..=ROM_N_END => panic!("READ-ONLY memory!!!"),
            0xe000..=0xfdff | 0xfea0..=0xfeff => panic!("Nintendo says no!!!"),
            0xFF04 => self.memory.set(address, 0),
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
    pub fn set_int_enable_joypad(&mut self, value: bool){
        let mut val =  self.get(0xFF0F);
        val.set_bit(4, value);
        self.set(0xFFFF, val);
    }
    pub fn set_int_enable_serial(&mut self, value: bool){
        let mut val =  self.get(0xFFFF);
        val.set_bit(3, value);
        self.set(0xFFFF, val);
    }
    pub fn set_int_enable_timer(&mut self, value: bool){
        let mut val =  self.get(0xFFFF);
        val.set_bit(2, value);
        self.set(0xFFFF, val);
    }
    pub fn set_int_enable_lcd(&mut self, value: bool){
        let mut val =  self.get(0xFFFF);
        val.set_bit(1, value);
        self.set(0xFFFF, val);
    }
    pub fn set_int_enable_vblank(&mut self, value: bool){
        let mut val =  self.get(0xFFFF);
        val.set_bit(0, value);
        self.set(0xFFFF, val);
    }
    pub fn get_int_enable_joypad(&self) -> bool{
        self.get(0xFFFF).bit(4)
    }
    pub fn get_int_enable_serial(&self) -> bool{
        self.get(0xFFFF).bit(3)
    }
    pub fn get_int_enable_timer(&self) -> bool{
        self.get(0xFFFF).bit(2)
    }
    pub fn get_int_enable_lcd(&self) -> bool{
        self.get(0xFFFF).bit(1)
    }
    pub fn get_int_enable_vblank(&self) -> bool{
        self.get(0xFFFF).bit(0)
    }
    pub fn set_int_request_joypad(&mut self, value: bool){
        let mut val =  self.get(0xFF0F);
        val.set_bit(4, value);
        self.set(0xFF0F, val);
    }
    pub fn set_int_request_serial(&mut self, value: bool){
        let mut val =  self.get(0xFF0F);
        val.set_bit(3, value);
        self.set(0xFF0F, val);
    }
    pub fn set_int_request_timer(&mut self, value: bool){
        let mut val =  self.get(0xFF0F);
        val.set_bit(2, value);
        self.set(0xFF0F, val);
    }
    pub fn set_int_request_lcd(&mut self, value: bool){
        let mut val =  self.get(0xFF0F);
        val.set_bit(1, value);
        self.set(0xFF0F, val);
    }
    pub fn set_int_request_vblank(&mut self, value: bool){
        let mut val =  self.get(0xFF0F);
        val.set_bit(0, value);
        self.set(0xFF0F, val);
    }
    pub fn get_int_request_joypad(&self) -> bool{
        self.get(0xFF0F).bit(4)
    }
    pub fn get_int_request_serial(&self) -> bool{
        self.get(0xFF0F).bit(3)
    }
    pub fn get_int_request_timer(&self) -> bool{
        self.get(0xFF0F).bit(2)
    }
    pub fn get_int_request_lcd(&self) -> bool{
        self.get(0xFF0F).bit(1)
    }
    pub fn get_int_request_vblank(&self) -> bool{
        self.get(0xFF0F).bit(0)
    }
    pub fn set_joypad_input(&mut self, bit: usize, value: bool){
        let mut val= self.get(0xFF00);
        val.set_bit(bit, !value);
        self.set(0xFF00, val)
    }
    pub fn load_rom(&mut self, buffer: Vec<u8>) {
        self.memory.load_rom(buffer);
    }
}


#[cfg(test)]
mod tests {
    use crate::bus::{Bus};

    #[test]
    fn rlc() {
        let mut bus = Bus::new();
        bus.set(0xB000, 0x80);

        let (z, n, h, c) = bus.rlc(false, false, 0, 0xB000);
        assert_eq!(z, false);
        assert_eq!(h, false);
        assert_eq!(c, true);
        assert_eq!(bus.get(0xB000), 0x01);
    }
    #[test]
    fn sra() {
        let mut bus = Bus::new();
        bus.set(0xB000, 0x01);

        let (z, n, h, c) = bus.sra(false, false, 0, 0xB000);
        assert_eq!(z, true);
        assert_eq!(h, false);
        assert_eq!(c, true);
        assert_eq!(bus.get(0xB000), 0x00);
    }
    #[test]
    fn rr() {
        let mut bus = Bus::new();

        bus.set(0xB000, 0x7C);

        let (z, n, h, c) = bus.rr(false, true, 0, 0xB000);
        assert_eq!(c, false);
        assert_eq!(bus.get(0xB000), 0xBE);

        bus.set(0xB000, 0x3D);

        let (z, n, h, c) = bus.rr(false, true, 0, 0xB000);
        assert_eq!(c, true);
        assert_eq!(bus.get(0xB000), 0x9E);

        bus.set(0xB000, 0xFF);

        let (z, n, h, c) = bus.rr(false, true, 0, 0xB000);
        assert_eq!(c, true);
        assert_eq!(bus.get(0xB000), 0xFF);

        bus.set(0xB000, 0x47);

        let (z, n, h, c) = bus.rr(false, false, 0, 0xB000);
        assert_eq!(c, true);
        assert_eq!(bus.get(0xB000), 0x23);

    }
}