const ROM_0: u16 = 0x0000;
const ROM_0_END: u16 = 0x3FFF;
const ROM_N: u16 = 0x4000;
const ROM_N_END: u16 = 0x7FFF;
const VRAM: u16 = 0x8000;
const VRAM_END: u16 = 0x9FFF;
const ERAM: u16 = 0xA000;
const ERAM_END: u16 = 0xBFFF;
const WRAM_0: u16 = 0xC000;
const WRAM_0_END: u16 = 0xCFFF;
const WRAM_N: u16 = 0xD000;
const WRAM_N_END: u16 = 0xDFFF;
const OAM: u16 = 0xFE00;
const OAM_END: u16 = 0xFE9F;
const IO_REGISTERS: u16 = 0xFF00;
const IO_REGISTERS_END: u16 = 0xFF7F;
const HRAM: u16 = 0xFF80;
const HRAM_END: u16 = 0xFFFE;
const INT_ENABLE: u16 = 0xFFFF;
const INT_ENABLE_END: u16 = 0xFFFF;
const INT_REQUEST: u16 = 0xFF0F;

const ROM_0_SIZE: u16 = ROM_0_END - ROM_0;
const ROM_N_SIZE: u16 = ROM_N_END - ROM_N;
const VRAM_SIZE: u16 = VRAM_END - VRAM;
const ERAM_SIZE: u16 = ERAM_END - ERAM;
const WRAM_0_SIZE: u16 = WRAM_0_END - WRAM_0;
const WRAM_N_SIZE: u16 = WRAM_N_END - WRAM_N;
const OAM_SIZE: u16 = OAM_END - OAM;
const IO_REGISTERS_SIZE: u16 = IO_REGISTERS_END - IO_REGISTERS;
const HRAM_SIZE: u16 = HRAM_END - HRAM;

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
}

impl Bus {
    pub fn new() -> Bus {
        Bus { rom_0: [0; ROM_0_SIZE as usize], rom_n: [0; ROM_N_SIZE as usize], vram: [0; VRAM_SIZE as usize], eram: [0; ERAM_SIZE as usize], wram_0: [0;  WRAM_0_SIZE as usize], wram_n: [0; WRAM_N_SIZE as usize], oam: [0; OAM_SIZE as usize], io_registers: [0; IO_REGISTERS_SIZE as usize], hram: [0; HRAM_SIZE as usize] }
    }
    
    pub fn get(&self, address: u16) -> u8 {
        match address { 
            ..=ROM_0_END => self.rom_0[(address - ROM_0) as usize],
            ROM_N..=ROM_N_END => self.rom_n[(address - ROM_N) as usize],
            VRAM..=VRAM_END => self.vram[(address - VRAM) as usize],
            ERAM..=ERAM_END => self.eram[(address - ERAM) as usize],
            WRAM_0..=WRAM_0_END => self.wram_0[(address - WRAM_0) as usize],
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
            _ => panic!("Not implemented yet!")
        };
        *target = value
    }
}
