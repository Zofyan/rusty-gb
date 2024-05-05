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

    pub fn arithmetic_flags(&self, base: u8, value: u8, func: fn(u8, u8) -> (u8, bool), carry: bool, carry_value: bool) -> (u8, bool, bool){
        let (mut c2, mut h2) = (false, false);
        let (mut val, c) = func(base, value);
        let (_, h) = func(base << 4, value << 4);
        if carry{
            (_, h2) = func(val << 4, (carry_value as u8) << 4);
            (val, c2) = func(val, carry_value as u8);
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
        let c = self.value.bit(7);
        self.value <<= 1;
        if !carry {
            self.value.set_bit(0, carry_value)
        } else{
            self.value.set_bit(0, c);
        }
        (false, false, false, c)
    }
    pub fn rotate_right_a(&mut self, carry: bool, carry_value: bool) -> (bool, bool, bool, bool) {
        let c = self.value.bit(0);
        self.value >>= 1;
        if !carry {
            self.value.set_bit(7, carry_value)
        } else{
            self.value.set_bit(7, c);
        }
        (false, false, false, c)
    }
    pub fn rl(&mut self, _: bool, carry_value: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        let c = self.value.bit(7);
        self.value <<= 1;
        self.value.set_bit(0, carry_value);
        (self.value == 0, false, false, c)
    }
    pub fn rlc(&mut self, _: bool, _: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        let c = self.value.bit(7);
        self.value <<= 1;
        self.value.set_bit(0, c);
        (self.value == 0, false, false, c)
    }
    pub fn rr(&mut self, _: bool, carry_value: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        let c = self.value.bit(0);
        self.value >>= 1;
        self.value.set_bit(7, carry_value);
        (self.value == 0, false, false, c)
    }
    pub fn rrc(&mut self, _: bool, _: bool, _: usize, _: u16) -> (bool, bool, bool, bool) {
        let c = self.value.bit(0);
        self.value >>= 1;
        self.value.set_bit(7, c);
        (self.value == 0, false, false, c)
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
        let bit7= self.get_bit(7);
        self.value >>= 1;
        self.value.set_bit(7, bit7);
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

#[cfg(test)]
mod tests {
    use crate::register::{Register};

    #[test]
    fn rlca() {
        let mut reg = Register { value: 0x80 };

        let (z, n, h, c) = reg.rotate_left_a(true, false);
        assert_eq!(z, false);
        assert_eq!(h, false);
        assert_eq!(c, true);
        assert_eq!(reg.value, 0x01);
    }
    #[test]
    fn rlc() {
        let mut reg = Register { value: 0x80 };

        let (z, n, h, c) = reg.rlc(false, false, 0, 0);
        assert_eq!(z, false);
        assert_eq!(h, false);
        assert_eq!(c, true);
        assert_eq!(reg.value, 0x01);
    }
    #[test]
    fn sra() {
        let mut reg = Register { value: 0x01 };

        let (z, n, h, c) = reg.sra(false, false, 0, 0);
        assert_eq!(z, true);
        assert_eq!(h, false);
        assert_eq!(c, true);
        assert_eq!(reg.value, 0x00);
    }
    #[test]
    fn add() {
        let mut reg = Register { value: 0xFE };

        let (z, n, h, c) = reg.add(01, true, true);
        assert_eq!(z, true);
        assert_eq!(h, true);
        assert_eq!(c, true);
        assert_eq!(reg.value, 0);
    }
    #[test]
    fn rr() {
        let mut reg = Register { value: 0 };

        reg.value = 0x7C;

        let (z, n, h, c) = reg.rr(false, true, 0, 0);
        assert_eq!(c, false);
        assert_eq!(reg.value, 0xBE);

        reg.value = 0x3D;

        let (z, n, h, c) = reg.rr(false, true, 0, 0);
        assert_eq!(c, true);
        assert_eq!(reg.value, 0x9E);

        reg.value = 0xFF;

        let (z, n, h, c) = reg.rr(false, true, 0, 0);
        assert_eq!(c, true);
        assert_eq!(reg.value, 0xFF);

        reg.value = 0x47;

        let (z, n, h, c) = reg.rr(false, false, 0, 0);
        assert_eq!(c, true);
        assert_eq!(reg.value, 0x23);

    }
}