use crate::bus::Bus;
use crate::register::Registers;


pub struct Cpu {
    pub bus: Bus,
    pub registers: Registers,
}

impl Cpu {
    // Constructor for Cpu
    pub fn new() -> Cpu {
        let bus = Bus::new();
        let registers = Registers::new();
        Cpu { bus, registers }
    }

    pub fn step(&self) {}

    fn _cycles(&self) {}

    fn load(&mut self, inst1: u8, inst2: u8) {
        let target_fn: fn(&mut Registers, u8) = match (inst1, inst2) {
            (0x4, ..=7) =>Registers::set_b,
            (0x4, 8..) =>Registers::set_c,
            (0x5, ..=7) =>Registers::set_d,
            (0x5, 8..) =>Registers::set_e,
            (0x6, ..=7) =>Registers::set_h,
            (0x6, 8..) =>Registers::set_l,
            (0x7, 8..) =>Registers::set_a,
            _ => panic!("Illegal instruction")
        };
        let source_fn: fn(&Registers) -> u8 = match (inst1, inst2) {
            (_, 0x0 | 0x8) =>Registers::get_b,
            (_, 0x1 | 0x9) =>Registers::get_c,
            (_, 0x2 | 0xa) =>Registers::get_d,
            (_, 0x3 | 0xb) =>Registers::get_e,
            (_, 0x4 | 0xc) =>Registers::get_h,
            (_, 0x5 | 0xd) =>Registers::get_l,
            (_, 0x7 | 0xf) =>Registers::get_a,
            _ => panic!("Illegal instruction")
        };
        let val = source_fn(&self.registers);
        target_fn(&mut self.registers, val);
    }
    fn load_into_memory(&mut self, inst1: u8, inst2: u8) {
        let target_fn: fn(&Registers) -> u16 = match (inst1, inst2) {
            (0x7, ..=5 | 0x7) =>Registers::get_hl,
            (0x0, 0x2) =>Registers::get_bc,
            (0x1, 0x2) =>Registers::get_de,
            (0x2 | 0x3, 0x2) =>Registers::get_hl,
            _ => panic!("Illegal instruction")
        };
        let source_fn: fn(&Registers) -> u8 = match (inst1, inst2) {
            (_, 0x0 | 0x8) =>Registers::get_b,
            (_, 0x1 | 0x9) =>Registers::get_c,
            (_, 0x2 | 0xa) =>Registers::get_d,
            (_, 0x3 | 0xb) =>Registers::get_e,
            (_, 0x4 | 0xc) =>Registers::get_h,
            (_, 0x5 | 0xd) =>Registers::get_l,
            (_, 0x7 | 0xf) =>Registers::get_a,
            (0x0..=0x3, 0x2) =>Registers::get_a,
            _ => panic!("Illegal instruction")
        };
        let val = source_fn(&mut self.registers);
        self.bus.set(target_fn(&self.registers), val);
        if inst1 == 0x2 {
            self.registers.set_hl(self.registers.get_hl() + 1);
        }
        if inst1 == 0x3 {
            self.registers.set_hl(self.registers.get_hl() - 1);
        }
    }
    fn load_from_memory(&mut self, inst1: u8, inst2: u8) {
        let target_fn: fn(&mut Registers, u8) = match (inst1, inst2) {
            (0x4, ..=7) =>Registers::set_b,
            (0x4, 8..) =>Registers::set_c,
            (0x5, ..=7) =>Registers::set_d,
            (0x5, 8..) =>Registers::set_e,
            (0x6, ..=7) =>Registers::set_h,
            (0x6, 8..) =>Registers::set_l,
            (0x7, 8..) =>Registers::set_a,
            (0x0..=0x3, 0xa) =>Registers::set_a,
            _ => panic!("Illegal instruction")
        };
        let source_fn: fn(&Registers) -> u16 = match (inst1, inst2) {
            (_, 0x6 | 0xe) =>Registers::get_hl,
            (0x0, 0xa) =>Registers::get_bc,
            (0x1, 0xa) =>Registers::get_de,
            (0x2 | 0x3, 0xa) =>Registers::get_hl,
            _ => panic!("Illegal instruction")
        };
        let val = self.bus.get(source_fn(&self.registers));
        target_fn(&mut self.registers, val);
        if inst1 == 0x2 {
            self.registers.set_hl(self.registers.get_hl() + 1);
        }
        if inst1 == 0x3 {
            self.registers.set_hl(self.registers.get_hl() - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu::Cpu;

    #[test]
    fn load() {
        let mut cpu = Cpu::new();
        cpu.registers.set_c(43);
        cpu.registers.set_d(223);

        cpu.load(0x5, 0x1);
        assert_eq!(cpu.registers.get_c(), 43);
        assert_eq!(cpu.registers.get_d(), 43);

        cpu.registers.set_hl(827);

        cpu.load_into_memory(0x2, 0x2);
        assert_eq!(cpu.bus.get(827), 43);

        cpu.registers.set_b(212);
        cpu.load_into_memory(0x7, 0x0);
        assert_eq!(cpu.bus.get(828), 212);

        cpu.registers.set_bc(828);
        cpu.load_from_memory(0x0, 0xa);
        assert_eq!(cpu.registers.get_a(), 212);
    }
}