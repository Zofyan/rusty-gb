use bitfield::{Bit, BitMut};

pub struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
}

impl Registers {
    pub fn new() -> Registers {
        Registers {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }
    pub fn get_flag_c(&self) -> bool {
        self.f.bit(4)
    }
    pub fn get_flag_h(&self) -> bool {
        self.f.bit(5)
    }
    pub fn get_flag_n(&self) -> bool {
        self.f.bit(6)
    }
    pub fn get_flag_z(&self) -> bool {
        self.f.bit(7)
    }
    pub fn set_flag_c(&mut self, value: bool) {
        self.f.set_bit(4, value)
    }
    pub fn set_flag_h(&mut self, value: bool) {
        self.f.set_bit(5, value)
    }
    pub fn set_flag_n(&mut self, value: bool) {
        self.f.set_bit(6, value)
    }
    pub fn set_flag_z(&mut self, value: bool)  {
        self.f.set_bit(7, value)
    }
    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | self.f as u16
    }
    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }
    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }
    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }
    pub fn get_a(&self) -> u8 {
        self.a
    }
    pub fn get_b(&self) -> u8 {
        self.b
    }
    pub fn get_c(&self) -> u8 {
        self.c
    }
    pub fn get_d(&self) -> u8 {
        self.d
    }
    pub fn get_e(&self) -> u8 {
        self.e
    }
    pub fn get_h(&self) -> u8 {
        self.h
    }
    pub fn get_l(&self) -> u8 {
        self.l
    }
    pub fn get_sp(&self) -> u16 {
        self.sp
    }
    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = value as u8
    }
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8
    }
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8
    }
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8
    }
    pub fn set_sp(&mut self, value: u16) {
        self.sp = value
    }
    pub fn set_pc(&mut self, value: u16) {
        self.pc = value
    }
    pub fn set_a(&mut self, value: u8) {
        self.a = value
    }
    pub fn set_b(&mut self, value: u8) {
        self.b = value
    }
    pub fn set_c(&mut self, value: u8) {
        self.c = value
    }
    pub fn set_d(&mut self, value: u8) {
        self.d = value
    }
    pub fn set_e(&mut self, value: u8) {
        self.e = value
    }
    pub fn set_h(&mut self, value: u8) {
        self.h = value
    }
    pub fn set_l(&mut self, value: u8) {
        self.l = value
    }
}

#[cfg(test)]
mod tests {
    use crate::register::Registers;

    #[test]
    fn basic() {
        let mut reg = Registers { a: 0, b: 0, c: 0, d: 0, e: 0, f: 0, h: 0, l: 0, sp: 0, pc: 0 };

        assert_eq!(reg.get_d(), 0);
        reg.set_d(34);
        assert_eq!(reg.get_d(), 34);

        reg.set_b(255);
        reg.set_c(255);
        assert_ne!(reg.get_bc(), (2u32.pow(16) - 2) as u16);
        assert_eq!(reg.get_bc(), (2u32.pow(16) - 1) as u16);

        reg.set_h(165);
        reg.set_l(98);
        assert_eq!(reg.get_hl(), (165 << 8) | 98);
    }
}