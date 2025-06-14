use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::ptr::null_mut;
use crate::bus::{ERAM, ERAM_END, ERAM_SIZE, HRAM, HRAM_END, HRAM_SIZE, INT_ENABLE, INT_ENABLE_END, INT_ENABLE_SIZE, IO_REGISTERS, IO_REGISTERS_END, IO_REGISTERS_SIZE, OAM, OAM_END, OAM_SIZE, ROM_0, ROM_0_END, ROM_0_SIZE, ROM_N, ROM_N_END, ROM_N_SIZE, VRAM, VRAM_END, VRAM_SIZE, WRAM_0, WRAM_0_END, WRAM_0_SIZE, WRAM_N, WRAM_N_END, WRAM_N_SIZE};

pub struct Memory {
    pub(crate) memory: [u8; 0x10000],
    pub(crate) eram: Vec<u8>,
    pub(crate) current_rom: usize,
    pub(crate) current_eram: usize,
    pub eram_enable: bool,
    pub(crate) banking_mode: u8,
    pub(crate) rom_address_cache: u16
}
impl Memory {
    pub fn new() -> Memory {
        Memory { memory: [0; 0x10000], eram: vec![], current_rom: 0, current_eram: 0, banking_mode: 0, eram_enable: false, rom_address_cache: 0 }
    }
    pub fn get(&self, address: u16) -> u8 {
        if address >= ERAM as u16 && address <= ERAM_END as u16 {
            self.eram[self.current_eram * ERAM_SIZE + (address as usize - ERAM)]
        } else{
            self.memory[address as usize]
        }
    }
    pub fn set(&mut self, address: u16, value: u8) {
        if address >= ERAM as u16 && address <= ERAM_END as u16 {
            self.eram[self.current_eram * ERAM_SIZE + (address as usize - ERAM)] = value;
        } else {
            self.memory[address as usize] = value
        }
    }
}