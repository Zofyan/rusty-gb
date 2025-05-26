use std::time::{Instant, SystemTime, UNIX_EPOCH};
use bitfield::Bit;
use bytesize::{kb, kib, ByteSize};
use crate::bus::ROM_N_SIZE;
use crate::memory::Memory;

pub trait MBC {
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {}
    fn read(&self, address: u16, memory: &Memory) -> u8 { memory.get(address) }
}

pub struct MBC0 {

}
impl MBC0 {
    pub fn new() -> Self {
        MBC0 {}
    }
}
impl MBC for MBC0 {
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {
        match address {
            ..=0x3FFF => {
                memory.current_rom = (value as u16 & 0b11111) | memory.current_rom & 0b01100000;
                if memory.current_rom & 0b11111 == 0 {
                    memory.current_rom = 1;
                }
                memory.rom_address_cache = (memory.current_rom as usize - 1) * ROM_N_SIZE as usize;
            },
            _ => {

            }
        }
    }
}

pub struct MBC2 {}
impl MBC2 {
    pub fn new() -> Self {
        MBC2 {}
    }
}
pub struct MBC1 {
    banking_mode: bool
}
impl MBC1 {
    pub fn new() -> Self {
        MBC1 { banking_mode: false }
    }
}
impl MBC for MBC1 {
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {
        match address {
            ..=0x1FFF => {
                memory.eram_enable = 0x0A == (value & 0x0F)
            },
            0x2000..=0x3FFF => {
                memory.current_rom = (value as u16 & 0b11111) | memory.current_rom & 0b01100000;
                if memory.current_rom & 0b11111 == 0 {
                    memory.current_rom = 1;
                }
                memory.rom_address_cache = (memory.current_rom as usize - 1) * ROM_N_SIZE as usize;
            },
            0x4000..=0x5FFF => {
                if memory.eram.len() >= ByteSize::kib(16).as_u64() as usize {
                    memory.current_eram = (value & 0b11) as u16;
                } else if memory.rom.len() >= ByteSize::mib(1).as_u64() as usize {
                    memory.current_rom = (value as u16 & 0b01100000) | memory.current_rom & 0b11111;
                }
                memory.rom_address_cache = (memory.current_rom as usize - 1) * ROM_N_SIZE as usize;
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
impl MBC for MBC2 {
    fn write(&mut self, address: u16, value: u8, memory: &mut Memory) {
        match address {
            ..=0x3FFF => {
                if address.bit(8) == false {
                    memory.eram_enable = 0x0A == (value & 0x0F)
                } else {
                    memory.current_rom = (value & 0b1111) as u16;
                    if memory.current_rom & 0b1111 == 0 {
                        memory.current_rom = 1;
                    }
                    memory.rom_address_cache = (memory.current_rom as usize - 1) * ROM_N_SIZE as usize;
                }
            },
            _ => {
                panic!("Not implemented for MBC1!")
            }
        }
    }
}

pub struct MBC3 {
    rtc_registers: bool,
    rtc_register: u8
}
impl MBC3 {
    pub fn new() -> Self {
        MBC3 { rtc_registers: false, rtc_register: 0x08 }
    }
}
impl MBC for MBC3 {
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
                memory.current_rom = (value & 0b1111111) as u16;
                if memory.current_rom & 0b1111111 == 0 {
                    memory.current_rom = 1;
                }
                memory.rom_address_cache = (memory.current_rom as usize - 1) * ROM_N_SIZE as usize;
            },
            0x4000..=0x5FFF => {
                if value <= 0x03 {
                    memory.current_eram = (value & 0b11) as u16;
                    memory.eram_enable = true;
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