use std::ptr::null_mut;
use crate::bus::{ERAM, ERAM_END, ERAM_SIZE, HRAM, HRAM_END, HRAM_SIZE, INT_ENABLE, INT_ENABLE_END, INT_ENABLE_SIZE, IO_REGISTERS, IO_REGISTERS_END, IO_REGISTERS_SIZE, OAM, OAM_END, OAM_SIZE, ROM_0, ROM_0_END, ROM_0_SIZE, ROM_N, ROM_N_END, ROM_N_SIZE, VRAM, VRAM_END, VRAM_SIZE, WRAM_0, WRAM_0_END, WRAM_0_SIZE, WRAM_N, WRAM_N_END, WRAM_N_SIZE};

pub struct Memory {
    pub(crate) rom: Vec<u8>,
    vram: Vec<u8>,
    pub(crate) eram: Vec<u8>,
    wram: Vec<u8>,
    oam: Vec<u8>,
    io_registers: Vec<u8>,
    hram: Vec<u8>,
    int_enable: Vec<u8>,
    extra_rom: Vec<Vec<u8>>,
    pub(crate) current_rom: u16,
    pub(crate) current_vram: u16,
    pub(crate) current_wram: u16,
    pub(crate) current_eram: u16,
    pub eram_enable: bool,
    pub(crate) banking_mode: u8,
    pub(crate) rom_address_cache: usize
}
impl Memory {
    pub fn new() -> Memory {
        Memory { rom: vec![0; (ROM_0_SIZE + ROM_N_SIZE) as usize], vram: vec![0; 2 * VRAM_SIZE as usize], eram: vec![0; ERAM_SIZE as usize], wram: vec![0; 8 * WRAM_0_SIZE as usize], oam: vec![0; OAM_SIZE as usize], io_registers: vec![0; IO_REGISTERS_SIZE as usize], hram: vec![0; HRAM_SIZE as usize], int_enable: vec![0; INT_ENABLE_SIZE as usize], extra_rom: vec![], current_rom: 0, current_eram: 0, current_wram: 1, current_vram: 0, banking_mode: 0, eram_enable: false, rom_address_cache: 0 }
    }
    pub fn get(&self, address: u16) -> u8 {
        match address {
            ..=ROM_0_END => self.rom[address as usize],
            ROM_N..=ROM_N_END => self.rom[self.rom_address_cache + address as usize],
            VRAM..=VRAM_END => self.vram[(self.current_vram * VRAM_SIZE + (address - VRAM)) as usize],
            ERAM..=ERAM_END => self.eram[(self.current_eram * ERAM_SIZE + (address - ERAM)) as usize],
            WRAM_0..=WRAM_0_END => self.wram[(address - WRAM_0) as usize],
            WRAM_N..=WRAM_N_END => self.wram[(self.current_wram * WRAM_N_SIZE + (address - WRAM_0)) as usize],
            OAM..=OAM_END => self.oam[(address - OAM) as usize],
            IO_REGISTERS..=IO_REGISTERS_END => self.io_registers[(address - IO_REGISTERS) as usize],
            HRAM..=HRAM_END => self.hram[(address - HRAM) as usize],
            INT_ENABLE..=INT_ENABLE_END => self.int_enable[(address - INT_ENABLE) as usize],
            _ => { 0xFF }
        }
    }
    pub fn set(&mut self, address: u16, value: u8) {
        let target = match address {
            ..=ROM_0_END => panic!("Read only memory, bug in MBC? {:#04x}", address),
            ROM_N..=ROM_N_END => panic!("Read only memory, bug in MBC? {:#04x}", address),
            VRAM..=VRAM_END => &mut self.vram[(self.current_vram * VRAM_SIZE + (address - VRAM)) as usize],
            ERAM..=ERAM_END => &mut self.eram[(self.current_eram * ERAM_SIZE + (address - ERAM)) as usize],
            WRAM_0..=WRAM_0_END => &mut self.wram[(address - WRAM_0) as usize],
            WRAM_N..=WRAM_N_END => &mut self.wram[(self.current_wram * WRAM_N_SIZE + (address - WRAM_0)) as usize],
            OAM..=OAM_END => &mut self.oam[(address - OAM) as usize],
            IO_REGISTERS..=IO_REGISTERS_END => &mut self.io_registers[(address - IO_REGISTERS) as usize],
            HRAM..=HRAM_END => &mut self.hram[(address - HRAM) as usize],
            INT_ENABLE..=INT_ENABLE_END => &mut self.int_enable[(address - INT_ENABLE) as usize],
            _ => panic!("Not implemented yet!")
        };
        *target = value
    }
    pub fn load_rom(&mut self, mut buffer: Vec<u8>) {
        self.rom[..=ROM_0_END as usize].copy_from_slice(&buffer.drain(..ROM_N_SIZE as usize).as_slice());
        self.current_rom = 1;

        match self.get(0x0149) {
            0x00 => {},
            0x02 => {
                self.eram.resize(1 * ERAM_SIZE as usize, 0);
                self.current_eram = 0;
            },
            0x03 => {
                self.eram.resize(4 * ERAM_SIZE as usize, 0);
                self.current_eram = 0;
            },
            _ => panic!("Not implemented yet! {}", self.get(0x0149))
        }
        match self.get(0x0148) {
            0x00 => {
                self.rom.resize(2 * ROM_N_SIZE as usize, 0);
                let start = ROM_N_SIZE as usize;
                let end = start + ROM_N_SIZE as usize;
                self.rom.get_mut(start..end).unwrap().copy_from_slice(&buffer.drain(..ROM_N_SIZE as usize).as_slice());

            },
            0x01 => {
                self.rom.resize(4 * ROM_N_SIZE as usize, 0);
                for i in 1..4 {
                    let start = i * ROM_N_SIZE as usize;
                    let end = start + ROM_N_SIZE as usize;
                    self.rom.get_mut(start..end).unwrap().copy_from_slice(&buffer.drain(..ROM_N_SIZE as usize).as_slice());
                }
            },
            0x03 => {
                self.rom.resize(16 * ROM_N_SIZE as usize, 0);
                for i in 1..16 {
                    let start = i * ROM_N_SIZE as usize;
                    let end = start + ROM_N_SIZE as usize;
                    self.rom.get_mut(start..end).unwrap().copy_from_slice(&buffer.drain(..ROM_N_SIZE as usize).as_slice());
                }
            },
            0x04 => {
                self.rom.resize(32 * ROM_N_SIZE as usize, 0);
                for i in 1..32 {
                    let start = i * ROM_N_SIZE as usize;
                    let end = start + ROM_N_SIZE as usize;
                    self.rom.get_mut(start..end).unwrap().copy_from_slice(&buffer.drain(..ROM_N_SIZE as usize).as_slice());
                }
            },
            0x05 => {
                self.rom.resize(64 * ROM_N_SIZE as usize, 0);
                for i in 1..64 {
                    let start = i * ROM_N_SIZE as usize;
                    let end = start + ROM_N_SIZE as usize;
                    self.rom.get_mut(start..end).unwrap().copy_from_slice(&buffer.drain(..ROM_N_SIZE as usize).as_slice());
                }
            }
            _ => panic!("Not implemented yet! {}", self.get(0x0148))
        }
    }
}