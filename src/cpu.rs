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

    pub fn step(&mut self) {
        if  !self.load(0, 0) &&
            !self.alu(0, 0) {
            panic!("Illegal instruction")
        }
    }

    fn _cycles(&self) {}

    fn load(&mut self, inst1: u8, inst2: u8) -> bool{
        let val: u8 = match (inst1, inst2) {
            (4..=7, 0x0 | 0x8) => self.registers.get_b(),
            (4..=7, 0x1 | 0x9) => self.registers.get_c(),
            (4..=7, 0x2 | 0xa) => self.registers.get_d(),
            (4..=7, 0x3 | 0xb) => self.registers.get_e(),
            (4..=7, 0x4 | 0xc) => self.registers.get_h(),
            (4..=7, 0x5 | 0xd) => self.registers.get_l(),
            (4..=7, 0x6 | 0xe) | (2..=3, 0xa) => self.bus.get(self.registers.get_hl()),
            (4..=7, 0x7 | 0xf) | (0..=3, 0x2) => self.registers.get_a(),
            (0x0, 0xa) => self.bus.get(self.registers.get_bc()),
            (0x1, 0xa) => self.bus.get(self.registers.get_de()),
            (0..=3, 0x6 | 0xe) => self.bus.get(self.registers.get_pc() + 1),
            _ => return false
        };
        match (inst1, inst2) {
            (0x4, ..=7) | (0, 0x6) => self.registers.set_b(val),
            (0x4, 8..) | (0, 0xe) => self.registers.set_c(val),
            (0x5, ..=7) | (1, 0x6) => self.registers.set_d(val),
            (0x5, 8..) | (1, 0xe) => self.registers.set_e(val),
            (0x6, ..=7) | (2, 0x6) => self.registers.set_h(val),
            (0x6, 8..) | (2, 0xe) => self.registers.set_l(val),
            (0x7, ..=5 | 7) | (2..=3, 0x2) | (3, 0x6) => self.bus.set(self.registers.get_hl(), val),
            (0x7, 8..) | (0..=3, 0xa) | (3, 0xe) => self.registers.set_a(val),
            (0x0, 0x2) => self.bus.set(self.registers.get_bc(), val),
            (0x1, 0x2) => self.bus.set(self.registers.get_de(), val),
            _ => return false
        };
        match (inst1, inst2) {
            (2, 2 | 0xa) => self.registers.set_hl(self.registers.get_hl() + 1),
            (3, 2 | 0xa) => self.registers.set_hl(self.registers.get_hl() - 1),
            _ => ()
        }
        true
    }

    pub fn alu(&mut self, inst1: u8, inst2: u8) -> bool{
        let old_carry_flag = self.registers.get_flag_c();
        let val = match (inst1, inst2) {
            (8..=0xb, 0 | 0x8) => self.registers.get_b(),
            (8..=0xb, 1 | 0x9) => self.registers.get_c(),
            (8..=0xb, 2 | 0xa) => self.registers.get_d(),
            (8..=0xb, 3 | 0xb) => self.registers.get_e(),
            (8..=0xb, 4 | 0xc) => self.registers.get_h(),
            (8..=0xb, 5 | 0xd) => self.registers.get_l(),
            (8..=0xb, 6 | 0xe) => self.bus.get(self.registers.get_hl()),
            (8..=0xb, 7 | 0xf) => self.registers.get_a(),
            (0, 4) => self.registers.get_b() + 1,
            (0, 5) => self.registers.get_b() - 1,
            (1, 4) => self.registers.get_d() + 1,
            (1, 5) => self.registers.get_d() - 1,
            (2, 4) => self.registers.get_h() + 1,
            (2, 5) => self.registers.get_h() - 1,
            (3, 4) => self.bus.get(self.registers.get_hl()) + 1,
            (3, 5) => self.bus.get(self.registers.get_hl()) - 1,
            (0, 0xc) => self.registers.get_c() + 1,
            (0, 0xd) => self.registers.get_c() - 1,
            (1, 0xc) => self.registers.get_e() + 1,
            (1, 0xd) => self.registers.get_e() - 1,
            (2, 0xc) => self.registers.get_l() + 1,
            (2, 0xd) => self.registers.get_l() - 1,
            (3, 0xc) => self.registers.get_a() + 1,
            (3, 0xd) => self.registers.get_a() - 1,
            (0xc..=0xf, 6 | 0xe) => self.bus.get(self.registers.get_pc() + 1),
            _ => return false
        };
        match (inst1, inst2) {
            (0x8, 0..=0x7) | (0xc, 6) => self.registers.add(val),
            (0x8, 8..=0xf) | (0xc, 0xe) => self.registers.addc(val),
            (0x9, 0..=0x7) | (0xd, 6) => self.registers.sub(val),
            (0x9, 8..=0xf) | (0xd, 0xe) => self.registers.subc(val),
            (0xa, 0..=0x7) | (0xe, 6) => self.registers.and(val),
            (0xa, 8..=0xf) | (0xe, 0xe) => self.registers.xor(val),
            (0xb, 0..=0x7) | (0xf, 6) => self.registers.or(val),
            (0xb, 8..=0xf) | (0xf, 0xe) => self.registers.cmp(val),
            (0, 4 | 5) => self.registers.set_b(val),
            (1, 4 | 5) => self.registers.set_d(val),
            (2, 4 | 5) => self.registers.set_h(val),
            (3, 4 | 5) => self.bus.set(self.registers.get_hl(), val),
            (0, 0xc | 0xd) => self.registers.set_c(val),
            (1, 0xc | 0xd) => self.registers.set_e(val),
            (2, 0xc | 0xd) => self.registers.set_l(val),
            (3, 0xc | 0xd) => self.registers.set_a(val),
            _ => return false
        }
        if inst1 <= 3 {
            self.registers.set_flag_c(old_carry_flag);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use crate::cpu::Cpu;

    #[test]
    fn load() {
        let mut cpu = Cpu::new();
        let mut rng = rand::thread_rng();

        let x1 = rng.gen_range(0..255);
        let x2 = rng.gen_range(0..255);
        let x3 = rng.gen_range(0..255);
        let x4 = rng.gen_range(0..255);
        let x5 = rng.gen_range(0..0xFFF);

        cpu.registers.set_c(x1);
        cpu.registers.set_d(x2);

        cpu.load(0x5, 0x1);
        assert_eq!(cpu.registers.get_c(), x1);
        assert_eq!(cpu.registers.get_d(), x1);

        cpu.registers.set_hl(x5);
        cpu.registers.set_a(x3);

        cpu.load(0x2, 0x2);
        assert_eq!(cpu.bus.get(x5), x3);

        cpu.registers.set_b(x4);
        cpu.load(0x7, 0x0);
        assert_eq!(cpu.bus.get(x5 + 1), x4);

        cpu.registers.set_bc(x5 + 1);
        cpu.load(0x0, 0xa);
        assert_eq!(cpu.registers.get_a(), x4);

        cpu.bus.set(cpu.registers.get_pc() + 1, x1);
        cpu.load(1, 0xe);
        assert_eq!(cpu.registers.get_e(), x1);
    }

    #[test]
    fn alu() {
        let mut cpu = Cpu::new();
        let mut rng = rand::thread_rng();

        let s1 = rng.gen_range(0..=7);
        let s2 = rng.gen_range(8..=15);
        let x1 = rng.gen_range(0..=255);
        let x2 = rng.gen_range(0..=255);
        let x3 = rng.gen_range(0..=255);
        let x4 = rng.gen_range(0..=127);
        let x5 = rng.gen_range(128..=255);
        let y1 = rng.gen_range(0..=0xFFF);

        cpu.registers.set_a(x1);
        cpu.registers.set_d(x2);
        cpu.registers.set_hl(y1);
        cpu.bus.set(y1, x3);

        cpu.alu(0x8, 0x2);
        assert_eq!(cpu.registers.get_a(), x1.wrapping_add(x2));

        cpu.registers.set_a(x1);
        cpu.alu(0x8, 0x6);
        assert_eq!(cpu.registers.get_a(), x1.wrapping_add(x3));

        cpu.registers.set_a(x1);
        cpu.alu(0x9, 0x2);
        assert_eq!(cpu.registers.get_a(), x1.wrapping_sub(x2));

        cpu.registers.set_a(x1);
        cpu.alu(0x9, 0x6);
        assert_eq!(cpu.registers.get_a(), x1.wrapping_sub(x3));

        cpu.registers.set_a(x1);
        cpu.registers.set_flag_c(false);
        cpu.alu(0x8, 0xa);
        assert_eq!(cpu.registers.get_a(), x1.wrapping_add(x2));

        cpu.registers.set_a(x1);
        cpu.registers.set_flag_c(true);
        cpu.alu(0x8, 0xe);
        assert_eq!(cpu.registers.get_a(), x1.wrapping_add(x3 + 1));

        cpu.registers.set_a(x1);
        cpu.registers.set_flag_c(false);
        cpu.alu(0x9, 0xa);
        assert_eq!(cpu.registers.get_a(), x1.wrapping_sub(x2));

        cpu.registers.set_a(x1);
        cpu.registers.set_flag_c(true);
        cpu.alu(0x9, 0xe);
        assert_eq!(cpu.registers.get_a(), x1.wrapping_sub(x3 + 1));

        cpu.registers.set_a(x1);
        cpu.alu(0xa, 0x2);
        assert_eq!(cpu.registers.get_a(), x1 & x2);

        cpu.registers.set_a(x1);
        cpu.alu(0xa, 0x6);
        assert_eq!(cpu.registers.get_a(), x1 & x3);

        cpu.registers.set_a(x1);
        cpu.alu(0xb, 0x2);
        assert_eq!(cpu.registers.get_a(), x1 | x2);

        cpu.registers.set_a(x1);
        cpu.alu(0xb, 0x6);
        assert_eq!(cpu.registers.get_a(), x1 | x3);

        cpu.registers.set_a(x1);
        cpu.alu(0xa, 0xa);
        assert_eq!(cpu.registers.get_a(), x1 ^ x2);

        cpu.registers.set_a(x1);
        cpu.alu(0xa, 0xe);
        assert_eq!(cpu.registers.get_a(), x1 ^ x3);

        cpu.registers.set_a(x4);
        cpu.registers.set_b(x5);
        cpu.alu(0x9, 0);
        assert_eq!(cpu.registers.get_flag_c(), true);
        assert_eq!(cpu.registers.get_flag_n(), true);
        assert_eq!(cpu.registers.get_flag_z(), x4 == x5);

        cpu.registers.set_a(x4);
        cpu.registers.set_b(x4);
        cpu.alu(0xb, 0x8);
        assert_eq!(cpu.registers.get_flag_n(), true);
        assert_eq!(cpu.registers.get_flag_z(), true);

        cpu.registers.set_a(s2);
        cpu.registers.set_b(s2);
        cpu.alu(0x8, 0);
        assert_eq!(cpu.registers.get_flag_n(), false);
        assert_eq!(cpu.registers.get_flag_h(), true);
    }
}