use crate::bus::{ERAM, ERAM_END, ERAM_SIZE, HRAM, HRAM_END, HRAM_SIZE, INT_ENABLE, INT_ENABLE_END, INT_ENABLE_SIZE, IO_REGISTERS, IO_REGISTERS_END, IO_REGISTERS_SIZE, OAM, OAM_END, OAM_SIZE, ROM_0, ROM_0_END, ROM_0_SIZE, ROM_N, ROM_N_END, ROM_N_SIZE, VRAM, VRAM_END, VRAM_SIZE, WRAM_0, WRAM_0_END, WRAM_0_SIZE, WRAM_N, WRAM_N_END, WRAM_N_SIZE};

pub struct Memory {
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


impl Memory {
    pub fn new() -> Memory {
        Memory { rom_0: [0; ROM_0_SIZE as usize], rom_n: [0; ROM_N_SIZE as usize], vram: [0; VRAM_SIZE as usize], eram: [0; ERAM_SIZE as usize], wram_0: [0; WRAM_0_SIZE as usize], wram_n: [0; WRAM_N_SIZE as usize], oam: [0; OAM_SIZE as usize], io_registers: [0; IO_REGISTERS_SIZE as usize], hram: [0; HRAM_SIZE as usize], int_enable: [0; INT_ENABLE_SIZE as usize] }
    }
    pub fn get(&self, address: u16) -> u8 {
        match address {
            ..=ROM_0_END => self.rom_0[(address - ROM_0) as usize],
            ROM_N..=ROM_N_END => self.rom_n[(address - ROM_N) as usize],
            VRAM..=VRAM_END => self.vram[(address - VRAM) as usize],
            ERAM..=ERAM_END => self.eram[(address - ERAM) as usize],
            WRAM_0..=WRAM_0_END => self.wram_0[(address - WRAM_0) as usize],
            WRAM_N..=WRAM_N_END => self.wram_n[(address - WRAM_N) as usize],
            OAM..=OAM_END => self.oam[(address - OAM) as usize],
            IO_REGISTERS..=IO_REGISTERS_END => self.io_registers[(address - IO_REGISTERS) as usize],
            HRAM..=HRAM_END => self.hram[(address - HRAM) as usize],
            INT_ENABLE..=INT_ENABLE_END => self.int_enable[(address - INT_ENABLE) as usize],
            _ => panic!("Not implemented yet!")
        }
    }
    pub fn set(&mut self, address: u16, value: u8) {
        let target = match address {
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
        };
        *target = value
    }
    pub fn load_rom(&mut self, buffer: Vec<u8>) {
        self.rom_0[..ROM_0_SIZE as usize].copy_from_slice(&buffer[..=ROM_0_END as usize]);
        self.rom_n[..ROM_N_SIZE as usize].copy_from_slice(&buffer[ROM_N as usize..=ROM_N_END as usize]);
    }
}