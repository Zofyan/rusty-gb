use bitfield::{Bit, BitMut};

pub struct Register {
    pub(crate) value: u8
}

impl Register {
    pub fn get(&self) -> u8 {
        self.value
    }
    pub fn set(&mut self, value: u8) {
        self.value = value
    }
    pub fn get_bit(&self, bit: usize) -> bool{
        self.value.bit(bit)
    }
    pub fn set_bit(&mut self, bit: usize, value: bool) {
        self.value.set_bit(bit, value)
    }

    pub(crate) fn arithmetic_flags(&self, base: u8, value: u8, func: fn(u8, u8) -> (u8, bool), carry: bool, carry_value: bool) -> (u8, bool, bool){
        let (mut c2, mut h2) = (false, false);
        let (mut val, c) = func(base, value);
        let (_, h) = func(base << 4, value << 4);
        if carry{
            (val, c2) = func(val, carry_value as u8);
            (_, h2) = func(val << 4, (carry_value as u8) << 4);
        }
        (val, c | c2, h | h2)
    }
    pub fn and(&mut self, value: u8) -> (bool, bool, bool, bool) {
        self.value &= value;
        (self.value == 0, false, true, false)
    }
    pub fn xor(&mut self, value: u8) -> (bool, bool, bool, bool) {
        self.value ^= value;
        (self.value == 0, false, false, false)
    }
    pub fn or(&mut self, value: u8) -> (bool, bool, bool, bool) {
        self.value |= value;
        (self.value == 0, false, false, false)
    }
    pub fn cp(&mut self, value: u8) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.arithmetic_flags(self.value, value, u8::overflowing_sub, false, false);
        (val == 0, true, h, c)
    }
    pub fn rotate_left_a(&mut self, carry: bool, carry_value: bool) -> (bool, bool, bool, bool) {
        if !carry {
            self.value.set_bit(0, carry_value)
        }
        self.value = self.value.rotate_left(1);
        (false, false, false, self.value.bit(0))
    }
    pub fn rotate_right_a(&mut self, carry: bool, carry_value: bool) -> (bool, bool, bool, bool) {
        if !carry {
            self.value.set_bit(7, carry_value)
        }
        self.value = self.value.rotate_right(1);
        (false, false, false, self.value.bit(7))
    }
    pub fn rl(&mut self, _: bool, carry_value: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        self.value.set_bit(0, carry_value);
        self.value = self.value.rotate_left(1);
        (self.value == 0, false, false, self.value.bit(0))
    }
    pub fn rlc(&mut self, _: bool, _: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        self.value = self.value.rotate_left(1);
        (self.value == 0, false, false, self.value.bit(0))
    }
    pub fn rr(&mut self, _: bool, carry_value: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        self.value.set_bit(7, carry_value);
        self.value = self.value.rotate_right(1);
        (self.value == 0, false, false, self.value.bit(7))
    }
    pub fn rrc(&mut self, _: bool, _: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        self.value = self.value.rotate_right(1);
        (self.value == 0, false, false, self.value.bit(7))
    }
    pub fn sla(&mut self, _: bool, _: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        let c= self.get_bit(7);
        self.value <<= 1;
        (self.value == 0, false, false, c)
    }
    pub fn srl(&mut self, _: bool, _: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        let c= self.get_bit(0);
        self.value >>= 1;
        (self.value == 0, false, false, c)
    }
    pub fn sra(&mut self, _: bool, _: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        let c= self.get_bit(0);
        self.value = (self.value >> 1) | self.value;
        (self.value == 0, false, false, c)
    }
    pub fn swap(&mut self, _: bool, _: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        self.value = (self.value << 4) | (self.value >> 4);
        (self.value == 0, false, false, false)
    }
    pub fn bit(&mut self, _: bool, _: bool, bit: usize, _: u16) -> (bool, bool, bool, bool) {
        (!self.value.bit(bit), false, true, false)
    }
    pub fn reset(&mut self, _: bool, _: bool, bit: usize, _: u16) -> (bool, bool, bool, bool) {
        self.value.set_bit(bit, false);
        (false, false, false, false)
    }
    pub fn setb(&mut self, _: bool, _: bool, bit: usize, _: u16) -> (bool, bool, bool, bool) {
        self.value.set_bit(bit, true);
        (false, false, false, false)
    }
    pub fn add(&mut self, value: u8, carry: bool, carry_value: bool) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.arithmetic_flags(self.value, value, u8::overflowing_add, carry, carry_value);
        self.value = val;
        (val == 0, false, h, c)
    }
    pub fn sub(&mut self, value: u8, carry: bool, carry_value: bool) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.arithmetic_flags(self.value, value, u8::overflowing_sub, carry, carry_value);
        self.value = val;
        (val == 0, true, h, c)
    }
    pub fn inc(&mut self) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.arithmetic_flags(self.value, 1, u8::overflowing_add, false, false);
        self.value = val;
        (val == 0, false, h, c)
    }
    pub fn dec(&mut self) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.arithmetic_flags(self.value, 1, u8::overflowing_sub, false, false);
        self.value = val;
        (val == 0, true, h, c)
    }
}
/*
#[cfg(test)]
mod tests {
    use crate::register::{Register, Registers};

    #[test]
    fn basic() {
        let mut reg = Registers { a: Register {value:0}, b: 0, c: 0, d: 0, e: 0, f: 0, h: 0, l: 0, sp: 0, pc: 0 };

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
}*/