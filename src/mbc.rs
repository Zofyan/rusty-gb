use std::time::{SystemTime, UNIX_EPOCH};
use bitfield::Bit;
use bytesize::{ByteSize};
use crate::bus::{ERAM, ERAM_END, ERAM_SIZE};
use crate::memory::Memory;
use crate::rom::Rom;

/// Cartridge mapper.
///
/// This was a `Box<dyn MBC>` generic over the ROM reader. On Cortex-M33 that
/// cost a vtable load out of flash plus an unpredictable `bx` on every read of
/// 0x0000-0x7FFF, i.e. on every opcode fetch, because the core has no branch
/// target buffer. As an enum the dispatch is a discriminant compare the
/// compiler can inline through, and the emulator holds no boxed state.
pub enum Mbc {
    Dummy,
    Mbc0,
    Mbc1 { banking_mode: bool, rom_size: usize },
    Mbc2,
    Mbc3 { rtc_registers: bool, rtc_register: u8 },
}

impl Mbc {
    /// Reads 0xA000-0xBFFF. Cartridge ROM does not come through here any more:
    /// every mapper returned `memory.get(address)` for 0x0000-0x7FFF, so `Bus`
    /// reads the `Rom` directly and skips the dispatch entirely.
    #[inline]
    pub fn read(&self, address: u16, memory: &Memory) -> u8 {
        match self {
            Mbc::Mbc3 { rtc_register, .. } if !memory.eram_enable => read_rtc(*rtc_register),
            _ => memory.get(address),
        }
    }

    /// Bank switching is rare next to reads, and inlining this whole match into
    /// `Bus::set` pushed `Bus::get`/`Bus::set` over the inline threshold, which
    /// cost far more than the call ever will.
    #[inline(never)]
    pub fn write(&mut self, address: u16, value: u8, memory: &mut Memory, rom: &mut Rom) {
        match self {
            Mbc::Dummy => {}
            Mbc::Mbc0 => match address {
                ..=0x3FFF => {
                    let previous = memory.current_rom;
                    memory.current_rom = ((value as u16 & 0b11111)
                        | memory.current_rom as u16 & 0b01100000)
                        as usize;
                    if memory.current_rom & 0b11111 == 0 {
                        memory.current_rom = 1;
                    }
                    if previous != memory.current_rom {
                        rom.select(memory.current_rom);
                    }
                }
                _ => {}
            },
            Mbc::Mbc1 { banking_mode, rom_size } => match address {
                ..=0x1FFF => memory.eram_enable = 0x0A == (value & 0x0F),
                0x2000..=0x3FFF => {
                    let previous = memory.current_rom;
                    memory.current_rom = ((value as u16 & 0b11111)
                        | memory.current_rom as u16 & 0b01100000)
                        as usize;
                    if memory.current_rom & 0b11111 == 0 {
                        memory.current_rom = 1;
                    }
                    if previous != memory.current_rom {
                        rom.select(memory.current_rom);
                    }
                }
                0x4000..=0x5FFF => {
                    if memory.eram.len() >= ByteSize::kib(16).as_u64() as usize {
                        memory.swap_eram((value & 0b11) as usize);
                    } else if *rom_size >= ByteSize::mib(1).as_u64() as usize {
                        let previous = memory.current_rom;
                        memory.current_rom = ((value as u16 & 0b01100000)
                            | memory.current_rom as u16 & 0b11111)
                            as usize;
                        if previous != memory.current_rom {
                            rom.select(memory.current_rom);
                        }
                    }
                }
                0x6000..=0x7FFF => *banking_mode = value & 0x1 == 1,
                _ => memory.set(address, value),
            },
            Mbc::Mbc2 => match address {
                ..=0x3FFF => {
                    if address.bit(8) == false {
                        memory.eram_enable = 0x0A == (value & 0x0F)
                    } else {
                        let previous = memory.current_rom;
                        memory.current_rom = (value & 0b1111) as usize;
                        if memory.current_rom & 0b1111 == 0 {
                            memory.current_rom = 1;
                        }
                        if previous != memory.current_rom {
                            rom.select(memory.current_rom);
                        }
                    }
                }
                _ => panic!("Not implemented for MBC1!"),
            },
            Mbc::Mbc3 { rtc_registers, rtc_register } => match address {
                ..=0x1FFF => {
                    memory.eram_enable = 0x0A == (value & 0x0F);
                    *rtc_registers = 0x0A == (value & 0x0F)
                }
                0x2000..=0x3FFF => {
                    let previous = memory.current_rom;
                    memory.current_rom = (value & 0b1111111) as usize;
                    if memory.current_rom & 0b1111111 == 0 {
                        memory.current_rom = 1;
                    }
                    // Games re-select the bank they already have loaded
                    // constantly; skipping those avoids paging it back in.
                    if previous != memory.current_rom {
                        rom.select(memory.current_rom);
                    }
                }
                0x4000..=0x5FFF => {
                    if value <= 0x07 {
                        memory.swap_eram((value & 0b11) as usize);
                        memory.eram_enable = true;
                    } else if value <= 0x0c && value >= 0x08 {
                        memory.eram_enable = false;
                        *rtc_register = value
                    }
                }
                0x6000..=0x7FFF => {}
                0xA000..=0xBFFF => {
                    if memory.eram_enable {
                        memory.set(address, value)
                    }
                }
                _ => panic!("Not implemented for MBC3! {:#04x}", address),
            },
        }
    }
}

/// Out of line and cold: it pulls in `SystemTime`, and letting it inline into
/// `Mbc::read` would in turn stop `Bus::get` from inlining into its callers.
#[cold]
#[inline(never)]
fn read_rtc(rtc_register: u8) -> u8 {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    match rtc_register {
        0x08 => (secs % 60) as u8,
        0x09 => ((secs / 60) % 60) as u8,
        0x0A => ((secs / 3600) % 24) as u8,
        0x0B => (((secs / 3600 / 24) % 512) & 0xFF) as u8,
        0x0C => (((secs / 3600 / 24) % 512) & 0x100) as u8,
        _ => panic!("no {:#02x}", rtc_register),
    }
}
