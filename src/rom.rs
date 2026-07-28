/// One ROM bank. The cartridge is addressed as a fixed bank at 0x0000-0x3FFF
/// plus a switchable bank at 0x4000-0x7FFF.
pub const BANK_SIZE: usize = 0x4000;

/// The cartridge image, addressed in place.
///
/// The emulator keeps no copy of the ROM inside its own address space, so a
/// bank switch is one pointer store rather than a 16 KiB memcpy, and the RAM
/// map starts at 0x8000 instead of 0x0000.
///
/// `data` is `&'static` because that is what the RP2350 actually offers: the
/// cartridge sits in memory-mapped XIP flash and costs no SRAM. Host builds get
/// the same shape by leaking the loaded image, which lives for the run anyway.
///
/// The two visible banks are held as fixed-size array references rather than
/// slices so that indexing them with a masked offset needs no bounds check, and
/// they are kept in a two-element array so selecting between them is a shift and
/// a mask rather than a branch -- Cortex-M33 has no branch predictor, so a
/// branch on every opcode fetch costs a pipeline refill.
pub struct Rom {
    banks: [&'static [u8; BANK_SIZE]; 2],
    data: &'static [u8],
    /// `bank_count - 1`. Cartridge sizes are powers of two, so masking with
    /// this mirrors an over-large bank number the way the hardware does.
    bank_mask: usize,
}

fn bank_of(data: &'static [u8], index: usize) -> &'static [u8; BANK_SIZE] {
    data[index * BANK_SIZE..(index + 1) * BANK_SIZE]
        .try_into()
        .expect("rom bank out of range")
}

impl Rom {
    /// Host build: load the cartridge and hand it a 'static lifetime.
    ///
    /// Leaking is deliberate. The image lives for the whole run either way, and
    /// it lets the host share [`Rom::mapped`]'s `&'static [u8]` shape with the
    /// Pico, where the cartridge really is static (XIP flash).
    #[cfg(not(target_os = "none"))]
    pub fn file(path: &str) -> Rom {
        let data = std::fs::read(path).expect("Could not open rom");
        Rom::mapped(alloc::boxed::Box::leak(data.into_boxed_slice()))
    }

    /// Wraps an image that is already addressable for its whole lifetime.
    pub fn mapped(data: &'static [u8]) -> Rom {
        let banks = data.len() / BANK_SIZE;
        assert!(banks >= 2 && banks.is_power_of_two(), "bad rom size {}", data.len());
        // Bank 1 is the one visible at 0x4000 out of reset.
        Rom { banks: [bank_of(data, 0), bank_of(data, 1)], data, bank_mask: banks - 1 }
    }

    /// Reads a cartridge address in 0x0000-0x7FFF.
    #[inline]
    pub fn read(&self, address: u16) -> u8 {
        // Both indices are masked, so neither lookup needs a bounds check.
        self.banks[(address >> 14) as usize & 1][(address as usize) & (BANK_SIZE - 1)]
    }

    /// Makes `bank` visible at 0x4000-0x7FFF.
    #[inline]
    pub fn select(&mut self, bank: usize) {
        self.banks[1] = bank_of(self.data, bank & self.bank_mask);
    }
}

#[cfg(test)]
static TEST_ROM: [u8; BANK_SIZE * 2] = [0; BANK_SIZE * 2];

#[cfg(test)]
impl Rom {
    /// A blank cartridge, for tests that only exercise RAM.
    pub fn test() -> Rom {
        Rom::mapped(&TEST_ROM)
    }
}
