use crate::bus::{ERAM, ERAM_END, ERAM_SIZE};

/// Lowest address held in RAM. Everything below it is cartridge ROM, which is
/// read straight out of [`crate::rom::Rom`] instead of being shadowed here --
/// that saves 32 KiB and, on the RP2350, means bank switching costs no copy.
pub const MEM_BASE: usize = 0x8000;
/// 0x8000-0xFFFF: VRAM, ERAM, WRAM, OAM, IO registers, HRAM.
pub const MEM_SIZE: usize = 0x10000 - MEM_BASE;

pub struct Memory {
    memory: [u8; MEM_SIZE],
    pub(crate) eram: Vec<u8>,
    pub(crate) current_rom: usize,
    pub(crate) current_eram: usize,
    pub eram_enable: bool,
    pub(crate) banking_mode: u8,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            memory: [0; MEM_SIZE],
            eram: vec![],
            current_rom: 1,
            current_eram: 0,
            banking_mode: 0,
            eram_enable: false,
        }
    }

    /// `address` must be >= [`MEM_BASE`]; cartridge reads go through `Rom`.
    ///
    /// Masking rather than subtracting `MEM_BASE` is deliberate: for an address
    /// in range the two are identical, but the mask is provably less than the
    /// array length, so there is no bounds check. Subtracting leaves LLVM unable
    /// to rule out a wrapped index, and the resulting check on every RAM access
    /// measured ~18%.
    #[inline]
    pub fn get(&self, address: u16) -> u8 {
        debug_assert!(address as usize >= MEM_BASE, "{:#06x} is not RAM", address);
        self.memory[(address as usize) & (MEM_SIZE - 1)]
    }

    #[inline]
    pub fn set(&mut self, address: u16, value: u8) {
        debug_assert!(address as usize >= MEM_BASE, "{:#06x} is not RAM", address);
        self.memory[(address as usize) & (MEM_SIZE - 1)] = value
    }

    /// Saves the live external RAM page and pages `bank` in.
    pub fn swap_eram(&mut self, bank: usize) {
        const LIVE: usize = ERAM - MEM_BASE;
        let off = self.current_eram * ERAM_SIZE;
        self.eram[off..off + ERAM_SIZE].copy_from_slice(&self.memory[LIVE..=ERAM_END - MEM_BASE]);
        self.current_eram = bank;
        let off = bank * ERAM_SIZE;
        self.memory[LIVE..=ERAM_END - MEM_BASE].copy_from_slice(&self.eram[off..off + ERAM_SIZE]);
    }
}
