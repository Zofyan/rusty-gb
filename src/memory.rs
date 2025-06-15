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
        self.memory[address as usize]
    }
    pub fn set(&mut self, address: u16, value: u8) {
        self.memory[address as usize] = value
    }
}