use std::io::{BufReader};
use std::time::{SystemTime, UNIX_EPOCH};
use bitfield::Bit;
use bytesize::{ByteSize};
use cloneable_file::CloneableFile;
use crate::bus::{ERAM, ERAM_END, ERAM_SIZE, ROM_N, ROM_N_END, ROM_N_SIZE};
use crate::memory::Memory;
use crate::rom::ROM;

pub trait MBC {
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {}
    fn read(&self, address: u16, memory: &Memory) -> u8 { memory.get(address) }
}

pub struct MBC_DUMMY { }

impl MBC for MBC_DUMMY { }
pub struct MBC0<R: ROM> {
    reader: R,
}
impl<R: ROM> MBC0<R> {
    pub fn new(rom: R) -> Self {
        MBC0 { reader: rom }
    }
}
impl<R: ROM> MBC for MBC0<R> {
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {
        match address {
            ..=0x3FFF => {
                memory.current_rom = ((value as u16 & 0b11111) | memory.current_rom as u16 & 0b01100000) as usize;
                if memory.current_rom & 0b11111 == 0 {
                    memory.current_rom = 1;
                }
                self.reader.read(memory.current_rom * ROM_N_SIZE, &mut memory.memory[ROM_N..=ROM_N_END]);
            },
            _ => {

            }
        }
    }
}

pub struct MBC2<R: ROM> {
    reader: R,
}
impl<R: ROM> MBC2<R> {
    pub fn new(rom: R) -> Self {
        MBC2 { reader: rom }
    }
}
pub struct MBC1<R: ROM> {
    reader: R,
    banking_mode: bool,
    rom_size: usize
}
impl<R: ROM> MBC1<R> {
    pub fn new(rom: R, rom_size: usize) -> Self {
        MBC1 { banking_mode: false, reader: rom, rom_size }
    }
}
impl<R: ROM> MBC for MBC1<R> {
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {
        match address {
            ..=0x1FFF => {
                memory.eram_enable = 0x0A == (value & 0x0F)
            },
            0x2000..=0x3FFF => {
                memory.current_rom = ((value as u16 & 0b11111) | memory.current_rom as u16 & 0b01100000) as usize;
                if memory.current_rom & 0b11111 == 0 {
                    memory.current_rom = 1;
                }
                memory.rom_address_cache = ((memory.current_rom - 1) * ROM_N_SIZE) as u16;
                self.reader.read(memory.current_rom * ROM_N_SIZE, &mut memory.memory[ROM_N..=ROM_N_END]);
            },
            0x4000..=0x5FFF => {
                if memory.eram.len() >= ByteSize::kib(16).as_u64() as usize {
                    memory.eram[memory.current_eram * ERAM_SIZE..(memory.current_eram + 1) * ERAM_SIZE].copy_from_slice(&memory.memory[ERAM..=ERAM_END]);
                    memory.current_eram = (value & 0b11) as usize;
                    memory.memory[ERAM..=ERAM_END].copy_from_slice(&memory.eram[memory.current_eram * ERAM_SIZE..(memory.current_eram + 1) * ERAM_SIZE]);
                } else if self.rom_size >= ByteSize::mib(1).as_u64() as usize {
                    memory.current_rom = ((value as u16 & 0b01100000) | memory.current_rom as u16 & 0b11111) as usize;
                    self.reader.read(memory.current_rom * ROM_N_SIZE, &mut memory.memory[ROM_N..=ROM_N_END]);
                }
            },
            0x6000..=0x7FFF => {
                self.banking_mode = value & 0x1 == 1;
            },
            _ => {
                memory.set(address, value);
            }
        }
    }
}
impl<R: ROM> MBC for MBC2<R> {
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {
        match address {
            ..=0x3FFF => {
                if address.bit(8) == false {
                    memory.eram_enable = 0x0A == (value & 0x0F)
                } else {
                    memory.current_rom = (value & 0b1111) as u16 as usize;
                    if memory.current_rom & 0b1111 == 0 {
                        memory.current_rom = 1;
                    }
                    self.reader.read(memory.current_rom * ROM_N_SIZE, &mut memory.memory[ROM_N..=ROM_N_END]);
                }
            },
            _ => {
                panic!("Not implemented for MBC1!")
            }
        }
    }
}

pub struct MBC3<R: ROM> {
    rom: R,
    rtc_registers: bool,
    rtc_register: u8
}
impl<R: ROM> MBC3<R> {
    pub fn new(rom: R) -> Self {
        MBC3 { rtc_registers: false, rtc_register: 0x08, rom }
    }
}
impl<R: ROM> MBC for MBC3<R> {
    fn read(&self, address: u16, memory: &Memory) -> u8 {
        match address {
            0xA000..=0xBFFF => {
                if memory.eram_enable {
                    memory.get(address)
                } else {
                    match self.rtc_register { 
                        0x08 => {
                            (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() % 60) as u8
                        }
                        0x09 => {
                            ((SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() / 60) % 60) as u8
                        }
                        0x0A => {
                            ((SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() / 3600) % 24) as u8
                        }
                        0x0B => {
                            (((SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() / 3600 / 24) % 512) & 0xFF) as u8
                        }
                        0x0C => {
                            (((SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() / 3600 / 24) % 512) & 0x100) as u8
                        }
                        _ => {
                            panic!("no {:#02x}", self.rtc_register)
                        }
                    }
                }

            }
            _ => {
                memory.get(address)
            }
        }
    }
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {
        match address {
            ..=0x1FFF => {
                memory.eram_enable = 0x0A == (value & 0x0F);
                self.rtc_registers = 0x0A == (value & 0x0F)
            },
            0x2000..=0x3FFF => {
                memory.current_rom = (value & 0b1111111) as usize;
                if memory.current_rom & 0b1111111 == 0 {
                    memory.current_rom = 1;
                }


                self.rom.read(memory.current_rom * ROM_N_SIZE, &mut memory.memory[ROM_N..=ROM_N_END]);
            },
            0x4000..=0x5FFF => {
                if value <= 0x07 {
                    memory.eram[memory.current_eram * ERAM_SIZE..(memory.current_eram + 1) * ERAM_SIZE].copy_from_slice(&memory.memory[ERAM..=ERAM_END]);

                    memory.current_eram = (value & 0b11) as usize;
                    memory.eram_enable = true;

                    memory.memory[ERAM..=ERAM_END].copy_from_slice(&memory.eram[memory.current_eram * ERAM_SIZE..(memory.current_eram + 1) * ERAM_SIZE]);
                } else if value <= 0x0c && value >= 0x08 {
                    memory.eram_enable = false;
                    self.rtc_register = value
                }
            },
            0x6000..=0x7FFF => {

            },
            0xA000..=0xBFFF => {
                if memory.eram_enable {
                    memory.set(address, value)
                } else {

                }
            }
            _ => {
                panic!("Not implemented for MBC3! {:#04x}", address)
            }
        }
    }
}