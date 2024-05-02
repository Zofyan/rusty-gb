use crate::bus::Bus;
use crate::register::Registers;

pub struct Cpu {
    pub bus: Bus,
    pub registers: Registers,
    counter: u32,
}

impl Cpu {
    // Constructor for Cpu
    pub fn new() -> Cpu {
        let bus = Bus::new();
        let mut registers = Registers::new();
        registers.set_sp(crate::bus::HRAM_END);
        registers.set_pc(0x0100);
        registers.set_bc(0x0013);
        registers.set_af(0x01B0);
        registers.set_de(0x00D8);
        registers.set_hl(0x014D);
        Cpu { bus, registers, counter: 0 }
    }

    fn log(&self) {
        println!("A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: 00:{:04X} ({:02X} {:02X} {:02X} {:02X})",
                 self.registers.get_a(),
                 self.registers.get_f(),
                 self.registers.get_b(),
                 self.registers.get_c(),
                 self.registers.get_d(),
                 self.registers.get_e(),
                 self.registers.get_h(),
                 self.registers.get_l(),
                 self.registers.get_sp(),
                 self.registers.get_pc(),
                 self.bus.get(self.registers.get_pc()),
                 self.bus.get(self.registers.get_pc() + 1),
                 self.bus.get(self.registers.get_pc() + 2),
                 self.bus.get(self.registers.get_pc() + 3)
        )
    }
    pub fn step(&mut self) {
        let opcode = self.bus.get(self.registers.get_pc());
        let inst = (opcode >> 4, opcode & 0xF);
        //println!("Executing {:#02x}({:#01x}, {:#01x}) at {:#02x}", opcode, inst.0, inst.1, self.registers.get_pc());
        if !self.load(inst) &&
            !self.alu(inst) &&
            !self.load16(inst) &&
            !self.alu16(inst) &&
            !self.jump(inst) &&
            !self.pop(inst) &&
            !self.misc(inst) &&
            !self.rotate(inst) &&
            !self.reset(inst) &&
            !self.call(inst) &&
            !self.ret(inst) &&
            !self.push(inst) {
            panic!("Not implemented yet {:#02x} at {:#02x}", opcode, self.registers.get_pc())
        }
        self.counter += 1;
        //if self.counter % 100 == 0 {
        //    println!("{}", self.counter);
        //self.log();
        //}
    }

    fn _cycles(&self, _count: usize) {}
    fn _pc(&mut self, count: u16) { self.registers.set_pc(self.registers.get_pc() + count) }
    fn misc(&mut self, inst: (u8, u8)) -> bool {
        match inst {
            (0, 0) => {}
            (1, 0) | (7, 6) => panic!("Stopped/Halted"),
            (0xf, 3) => {}
            (0xf, 0xb) => (),
            _ => return false
        }
        self._cycles(1);
        self._pc(1);
        true
    }
    fn prefix(&mut self, inst: (u8, u8)) -> bool {
        if inst != (0xc, 0xb){
            return false
        }
        let opcode = self.bus.get(self.registers.get_pc() + 1);
        let inst = (opcode >> 4, opcode & 0xF);
        true
    }
    fn call(&mut self, inst: (u8, u8)) -> bool {
        let new_pc = match inst {
            (0xc..=0xd, 4 | 0xc) | (0xc, 0xd) => self.bus.get16(self.registers.get_pc() + 1),
            _ => return false
        };
        let condition = match inst {
            (0xc, 4) => !self.registers.get_flag_z(),
            (0xd, 4) => !self.registers.get_flag_c(),
            (0xc, 0xc) => self.registers.get_flag_z(),
            (0xd, 0xc) => self.registers.get_flag_c(),
            _ => true
        };
        self._pc(3);
        if condition {
            self._push(self.registers.get_pc());
            self.registers.set_pc(new_pc);
            self._cycles(3);
        }
        self._cycles(3);
        true
    }
    fn ret(&mut self, inst: (u8, u8)) -> bool {
        let condition = match inst {
            (0xc, 0) => !self.registers.get_flag_z(),
            (0xd, 0) => !self.registers.get_flag_c(),
            (0xc, 0x8) => self.registers.get_flag_z(),
            (0xd, 0x8) => self.registers.get_flag_c(),
            (0xc, 0x9) => true,
            _ => return false
        };
        self._pc(1);
        if condition {
            let new_pc = self._pop();
            self.registers.set_pc(new_pc);
            if inst == (0xc, 0x9) {
                self._cycles(4);
            } else{
                self._cycles(5);
            }
        } else{
            self._cycles(2);
        }
        true
    }
    fn reset(&mut self, inst: (u8, u8)) -> bool {
        let new_pc = match inst {
            (0xc, 7) => self.bus.get16(0x0),
            (0xc, 0xf) => self.bus.get16(0x8),
            (0xd, 7) => self.bus.get16(0x10),
            (0xd, 0xf) => self.bus.get16(0x18),
            (0xe, 7) => self.bus.get16(0x20),
            (0xe, 0xf) => self.bus.get16(0x28),
            (0xf, 7) => self.bus.get16(0x30),
            (0xf, 0xf) => self.bus.get16(0x38),
            _ => return false
        };
        self._pc(1);
        self._push(self.registers.get_pc());
        self.registers.set_pc(new_pc);
        true
    }
    fn rotate(&mut self, inst: (u8, u8)) -> bool {
        match inst {
            (0, 7) => self.registers.rotate_left_a(true),
            (1, 7) => self.registers.rotate_left_a(false),
            (0, 0xf) => self.registers.rotate_right_a(true),
            (1, 0xf) => self.registers.rotate_right_a(false),
            _ => return false
        }
        self._cycles(1);
        self._pc(1);
        true
    }
    fn _pop(&mut self) -> u16 {
        let val = self.bus.get16(self.registers.get_sp());
        self.registers.inc16_sp();
        self.registers.inc16_sp();
        val
    }
    fn pop(&mut self, inst: (u8, u8)) -> bool {
        let val = match inst {
            (0xc..=0xf, 1) => self._pop(),
            _ => return false
        };

        match inst {
            (0xc, 1) => self.registers.set_bc(val),
            (0xd, 1) => self.registers.set_de(val),
            (0xe, 1) => self.registers.set_hl(val),
            (0xf, 1) => self.registers.set_af(val),
            _ => return false
        }
        self._cycles(3);
        self._pc(1);
        true
    }
    fn _push(&mut self, value: u16) {
        self.bus.set16(self.registers.get_sp() - 2, value);
        self.registers.dec16_sp();
        self.registers.dec16_sp();
    }
    fn push(&mut self, inst: (u8, u8)) -> bool {
        let value = match inst {
            (0xc, 5) => self.registers.get_bc(),
            (0xd, 5) => self.registers.get_de(),
            (0xe, 5) => self.registers.get_hl(),
            (0xf, 5) => self.registers.get_af(),
            _ => return false
        };
        self._push(value);

        self._cycles(4);
        self._pc(1);
        true
    }
    fn load16(&mut self, inst: (u8, u8)) -> bool {
        let val: u16 = match inst {
            (0..=3, 1) => self.bus.get16(self.registers.get_pc() + 1),
            (0, 8) => self.registers.get_sp(),
            (0xf, 9) => self.registers.get_hl(),
            _ => return false
        };
        match inst {
            (0, 1) => self.registers.set_bc(val),
            (1, 1) => self.registers.set_de(val),
            (2, 1) => self.registers.set_hl(val),
            (3, 1) => self.registers.set_sp(val),
            (0, 8) => self.bus.set16(self.bus.get16(self.registers.get_pc() + 1), val),
            (0xf, 9) => self.registers.set_sp(val),
            _ => panic!("Illegal instruction")
        }
        match inst {
            (0, 8) => {
                self._cycles(5);
                self._pc(3)
            }
            (0xf, 9) => {
                self._cycles(2);
                self._pc(1)
            }
            _ => {
                self._cycles(3);
                self._pc(3)
            }
        }
        true
    }
    fn load(&mut self, inst: (u8, u8)) -> bool {
        let val: u8 = match inst {
            (4..=7, 0x0 | 0x8) => self.registers.get_b(),
            (4..=7, 0x1 | 0x9) => self.registers.get_c(),
            (4..=7, 0x2 | 0xa) => self.registers.get_d(),
            (4..=7, 0x3 | 0xb) => self.registers.get_e(),
            (4..=7, 0x4 | 0xc) => self.registers.get_h(),
            (4..=7, 0x5 | 0xd) => self.registers.get_l(),
            (4..=7, 0x6 | 0xe) | (2..=3, 0xa) => self.bus.get(self.registers.get_hl()),
            (4..=7, 0x7 | 0xf) | (0..=3, 0x2) | (0xe, 0 | 2 | 0xa) => self.registers.get_a(),
            (0x0, 0xa) => self.bus.get(self.registers.get_bc()),
            (0x1, 0xa) => self.bus.get(self.registers.get_de()),
            (0..=3, 0x6 | 0xe) => self.bus.get(self.registers.get_pc() + 1),
            (0xf, 0xa) => self.bus.get(self.bus.get16(self.registers.get_pc() + 1)),
            (0xf, 0) => self.bus.get(self.bus.get(self.registers.get_pc() + 1) as u16 + 0xFF00),
            (0xf, 2) => self.bus.get(self.registers.get_c() as u16 + 0xFF00),
            _ => return false
        };
        match inst {
            (0x4, ..=7) | (0, 0x6) => self.registers.set_b(val),
            (0x4, 8..) | (0, 0xe) => self.registers.set_c(val),
            (0x5, ..=7) | (1, 0x6) => self.registers.set_d(val),
            (0x5, 8..) | (1, 0xe) => self.registers.set_e(val),
            (0x6, ..=7) | (2, 0x6) => self.registers.set_h(val),
            (0x6, 8..) | (2, 0xe) => self.registers.set_l(val),
            (0x7, ..=5 | 7) | (2..=3, 0x2) | (3, 0x6) => self.bus.set(self.registers.get_hl(), val),
            (0x7, 8..) | (0..=3, 0xa) | (3, 0xe) | (0xf, 0 | 2 | 0xa) => self.registers.set_a(val),
            (0x0, 0x2) => self.bus.set(self.registers.get_bc(), val),
            (0x1, 0x2) => self.bus.set(self.registers.get_de(), val),
            (0xe, 0xa) => self.bus.set(self.bus.get16(self.registers.get_pc() + 1), val),
            (0xe, 0) => self.bus.set(self.bus.get(self.registers.get_pc() + 1) as u16 + 0xFF00, val),
            (0xe, 2) => self.bus.set(self.registers.get_c() as u16 + 0xFF00, val),
            _ => return false
        };
        match inst {
            (2, 2 | 0xa) => self.registers.set_hl(self.registers.get_hl() + 1),
            (3, 2 | 0xa) => self.registers.set_hl(self.registers.get_hl() - 1),
            _ => ()
        }
        match inst {
            (0..=3, 2 | 0xa) | (4..=7, 6 | 0xe) | (7, 0..=7) | (0xe..=0xf, 2) => {
                self._cycles(2);
                self._pc(1)
            }
            (0..=2, 6) | (0..=3, 0xe) => {
                self._cycles(2);
                self._pc(2)
            }
            (3, 6) | (0xe..=0xf, 0) => {
                self._cycles(3);
                self._pc(2)
            }
            (0xe..=0xf, 0xa) => {
                self._cycles(4);
                self._pc(3)
            }
            _ => {
                self._cycles(1);
                self._pc(1)
            }
        }
        true
    }
    fn alu16(&mut self, inst: (u8, u8)) -> bool {
        let val = match inst {
            (0, 9) => self.registers.get_bc(),
            (1, 9) => self.registers.get_de(),
            (2, 9) => self.registers.get_hl(),
            (3, 9) => self.registers.get_sp(),
            _ => 0
        };
        match inst {
            (0, 3) => self.registers.inc16_bc(),
            (1, 3) => self.registers.inc16_de(),
            (2, 3) => self.registers.inc16_hl(),
            (3, 3) => self.registers.inc16_sp(),
            (0, 0xb) => self.registers.dec16_bc(),
            (1, 0xb) => self.registers.dec16_de(),
            (2, 0xb) => self.registers.dec16_hl(),
            (3, 0xb) => self.registers.dec16_sp(),
            (0..=3, 9) => self.registers.add16(val),
            _ => return false
        }
        self._cycles(2);
        self._pc(1);
        true
    }
    fn alu(&mut self, inst: (u8, u8)) -> bool {
        let old_carry_flag = self.registers.get_flag_c();
        let mut val = match inst {
            (8..=0xb, 0 | 0x8) => self.registers.get_b(),
            (8..=0xb, 1 | 0x9) => self.registers.get_c(),
            (8..=0xb, 2 | 0xa) => self.registers.get_d(),
            (8..=0xb, 3 | 0xb) => self.registers.get_e(),
            (8..=0xb, 4 | 0xc) => self.registers.get_h(),
            (8..=0xb, 5 | 0xd) => self.registers.get_l(),
            (8..=0xb, 6 | 0xe) => self.bus.get(self.registers.get_hl()),
            (8..=0xb, 7 | 0xf) => self.registers.get_a(),
            (0..=3, 4 | 5 | 0xc | 0xd) => 0,
            (0xc..=0xf, 6 | 0xe) => self.bus.get(self.registers.get_pc() + 1),
            _ => return false
        };
        match inst {
            (0x8, 0..=0x7) | (0xc, 6) => self.registers.add(val),
            (0x8, 8..=0xf) | (0xc, 0xe) => self.registers.addc(val),
            (0x9, 0..=0x7) | (0xd, 6) => self.registers.sub(val),
            (0x9, 8..=0xf) | (0xd, 0xe) => self.registers.subc(val),
            (0xa, 0..=0x7) | (0xe, 6) => self.registers.and(val),
            (0xa, 8..=0xf) | (0xe, 0xe) => self.registers.xor(val),
            (0xb, 0..=0x7) | (0xf, 6) => self.registers.or(val),
            (0xb, 8..=0xf) | (0xf, 0xe) => self.registers.cmp(val),
            (0, 4) => self.registers.inc_b(),
            (0, 5) => self.registers.dec_b(),
            (1, 4) => self.registers.inc_d(),
            (1, 5) => self.registers.dec_d(),
            (2, 4) => self.registers.inc_h(),
            (2, 5) => self.registers.dec_h(),
            (3, 4) => self.registers.inc_m(self.bus.get_target_mut(self.registers.get_hl())),
            (3, 5) => self.registers.dec_m(self.bus.get_target_mut(self.registers.get_hl())),
            (0, 0xc) => self.registers.inc_c(),
            (0, 0xd) => self.registers.dec_c(),
            (1, 0xc) => self.registers.inc_e(),
            (1, 0xd) => self.registers.dec_e(),
            (2, 0xc) => self.registers.inc_l(),
            (2, 0xd) => self.registers.dec_l(),
            (3, 0xc) => self.registers.inc_a(),
            (3, 0xd) => self.registers.dec_a(),
            _ => return false
        }
        if inst.0 <= 3 {
            self.registers.set_flag_c(old_carry_flag);
        }
        match inst {
            (0xc..=0xf, 6 | 0xe) => {
                self._cycles(2);
                self._pc(2)
            }
            (8..=0xb, 6 | 0xe) => {
                self._cycles(2);
                self._pc(1)
            }
            _ => {
                self._cycles(1);
                self._pc(1)
            }
        }
        true
    }
    fn jump(&mut self, inst: (u8, u8)) -> bool {
        let condition = match inst {
            (2, 0) | (0xc, 2) => !self.registers.get_flag_z(),
            (2, 8) | (0xc, 0xa) => self.registers.get_flag_z(),
            (3, 0) | (0xd, 2) => !self.registers.get_flag_c(),
            (3, 8) | (0xd, 0xa) => self.registers.get_flag_c(),
            _ => true
        };
        let pc = self.registers.get_pc();
        if condition {
            match inst {
                (2..=3, 0) | (1..=3, 8) => self.registers.set_pc(pc.wrapping_add_signed(self.bus.gets(pc + 1) as i16) + 2),
                (0xc..=0xd, 2) | (0xc..=0xd, 0xa) | (0xc, 0x3) => {
                    self.registers.set_pc(self.bus.get16(pc + 1));
                    self._cycles(1)
                }
                _ => return false
            }
            self._cycles(3)
        } else {
            if inst.0 <= 3 {
                self._pc(2)
            } else {
                self._pc(3);
                self._cycles(1)
            }
            self._cycles(2)
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use crate::cpu::Cpu;

    #[test]
    #[should_panic]
    fn not_implemented() {
        let mut cpu = Cpu::new();
        cpu.bus.set(cpu.registers.get_pc(), 0xe8);
        cpu.step();
    }

    #[test]
    fn jump() {
        let mut cpu = Cpu::new();
        let mut rng = rand::thread_rng();

        let x1 = rng.gen_range(0..255);
        let y1 = rng.gen_range(0..0xFFFF);

        for _ in [..=30] {
            cpu.registers.set_pc(y1);
            cpu.bus.set(y1 + 1, x1);

            cpu.jump((0x1, 0x8));
            assert_eq!(cpu.registers.get_pc(), y1.wrapping_add_signed((x1 as i8) as i16) + 2);
        }
    }

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

        cpu.load((0x5, 0x1));
        assert_eq!(cpu.registers.get_c(), x1);
        assert_eq!(cpu.registers.get_d(), x1);

        cpu.registers.set_hl(x5);
        cpu.registers.set_a(x3);

        cpu.load((0x2, 0x2));
        assert_eq!(cpu.bus.get(x5), x3);

        cpu.registers.set_b(x4);
        cpu.load((0x7, 0x0));
        assert_eq!(cpu.bus.get(x5 + 1), x4);

        cpu.registers.set_bc(x5 + 1);
        cpu.load((0x0, 0xa));
        assert_eq!(cpu.registers.get_a(), x4);

        cpu.bus.set(cpu.registers.get_pc() + 1, x1);
        cpu.load((1, 0xe));
        assert_eq!(cpu.registers.get_e(), x1);
    }

    #[test]
    fn rotate_a() {
        let mut cpu = Cpu::new();

        let x1 = 0b01010101;
        let x2 = 0b10101010;

        cpu.registers.set_a(x1);
        cpu.rotate((0, 7));
        assert_eq!(cpu.registers.get_a(), x2);
        assert_eq!(cpu.registers.get_flag_c(), false);

        cpu.registers.set_a(x1);
        cpu.rotate((0, 0xf));
        assert_eq!(cpu.registers.get_a(), x2);
        assert_eq!(cpu.registers.get_flag_c(), true);

        cpu.registers.set_a(x2);
        cpu.rotate((0, 7));
        assert_eq!(cpu.registers.get_a(), x1);
        assert_eq!(cpu.registers.get_flag_c(), true);

        cpu.registers.set_a(x2);
        cpu.rotate((0, 0xf));
        assert_eq!(cpu.registers.get_a(), x1);
        assert_eq!(cpu.registers.get_flag_c(), false);
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

        cpu.alu((0x8, 0x2));
        assert_eq!(cpu.registers.get_a(), x1.wrapping_add(x2));

        cpu.registers.set_a(x1);
        cpu.alu((0x8, 0x6));
        assert_eq!(cpu.registers.get_a(), x1.wrapping_add(x3));

        cpu.registers.set_a(x1);
        cpu.alu((0x9, 0x2));
        assert_eq!(cpu.registers.get_a(), x1.wrapping_sub(x2));

        cpu.registers.set_a(x1);
        cpu.alu((0x9, 0x6));
        assert_eq!(cpu.registers.get_a(), x1.wrapping_sub(x3));

        cpu.registers.set_a(x1);
        cpu.registers.set_flag_c(false);
        cpu.alu((0x8, 0xa));
        assert_eq!(cpu.registers.get_a(), x1.wrapping_add(x2));

        cpu.registers.set_a(x1);
        cpu.registers.set_flag_c(true);
        cpu.alu((0x8, 0xe));
        assert_eq!(cpu.registers.get_a(), x1.wrapping_add(x3 + 1));

        cpu.registers.set_a(x1);
        cpu.registers.set_flag_c(false);
        cpu.alu((0x9, 0xa));
        assert_eq!(cpu.registers.get_a(), x1.wrapping_sub(x2));

        cpu.registers.set_a(x1);
        cpu.registers.set_flag_c(true);
        cpu.alu((0x9, 0xe));
        assert_eq!(cpu.registers.get_a(), x1.wrapping_sub(x3 + 1));

        cpu.registers.set_a(x1);
        cpu.alu((0xa, 0x2));
        assert_eq!(cpu.registers.get_a(), x1 & x2);

        cpu.registers.set_a(x1);
        cpu.alu((0xa, 0x6));
        assert_eq!(cpu.registers.get_a(), x1 & x3);

        cpu.registers.set_a(x1);
        cpu.alu((0xb, 0x2));
        assert_eq!(cpu.registers.get_a(), x1 | x2);

        cpu.registers.set_a(x1);
        cpu.alu((0xb, 0x6));
        assert_eq!(cpu.registers.get_a(), x1 | x3);

        cpu.registers.set_a(x1);
        cpu.alu((0xa, 0xa));
        assert_eq!(cpu.registers.get_a(), x1 ^ x2);

        cpu.registers.set_a(x1);
        cpu.alu((0xa, 0xe));
        assert_eq!(cpu.registers.get_a(), x1 ^ x3);

        cpu.registers.set_a(x4);
        cpu.registers.set_b(x5);
        cpu.alu((0x9, 0));
        assert_eq!(cpu.registers.get_flag_c(), true);
        assert_eq!(cpu.registers.get_flag_n(), true);
        assert_eq!(cpu.registers.get_flag_z(), x4 == x5);

        cpu.registers.set_a(x4);
        cpu.registers.set_b(x4);
        cpu.alu((0xb, 0x8));
        assert_eq!(cpu.registers.get_flag_n(), true);
        assert_eq!(cpu.registers.get_flag_z(), true);

        cpu.registers.set_a(s2);
        cpu.registers.set_b(s2);
        cpu.alu((0x8, 0));
        assert_eq!(cpu.registers.get_flag_n(), false);
        assert_eq!(cpu.registers.get_flag_h(), true);
    }

    #[test]
    fn inc() {
        let mut cpu = Cpu::new();

        let x1 = 0x1;
        let x2 = 0xff;
        let x3 = 0x0f;

        cpu.registers.set_c(x1);
        cpu.alu((0, 0xc));
        assert_eq!(cpu.registers.get_flag_z(), false);
        assert_eq!(cpu.registers.get_flag_n(), false);

        cpu.registers.set_c(x2);
        cpu.alu((0, 0xc));
        assert_eq!(cpu.registers.get_flag_z(), true);
        assert_eq!(cpu.registers.get_flag_n(), false);

        cpu.registers.set_c(x3);
        cpu.alu((0, 0xc));
        assert_eq!(cpu.registers.get_flag_h(), true);
        assert_eq!(cpu.registers.get_flag_n(), false);
    }

    #[test]
    fn dec() {
        let mut cpu = Cpu::new();

        let x1 = 0x1;
        let x2 = 0xff;
        let x3 = 0x00;

        cpu.registers.set_c(x2);
        cpu.alu((0, 0xd));
        assert_eq!(cpu.registers.get_flag_z(), false);
        assert_eq!(cpu.registers.get_flag_n(), true);

        cpu.registers.set_c(x1);
        cpu.alu((0, 0xd));
        assert_eq!(cpu.registers.get_flag_z(), true);
        assert_eq!(cpu.registers.get_flag_n(), true);

        cpu.registers.set_c(x3);
        cpu.alu((0, 0xd));
        assert_eq!(cpu.registers.get_flag_h(), true);
        assert_eq!(cpu.registers.get_flag_n(), true);
    }

    #[test]
    fn stack() {
        let mut cpu = Cpu::new();
        let mut rng = rand::thread_rng();

        let x1 = rng.gen_range(0..=255);
        let x2 = rng.gen_range(0..=255);
        let y1 = rng.gen_range(0..=0xFFFF);
        let y2 = rng.gen_range(0..=0xFFFF);

        cpu.registers.set_bc(y1);
        cpu.push((0xc, 5));
        cpu.pop((0xd, 1));
        assert_eq!(cpu.registers.get_de(), y1);

        cpu.registers.set_bc(y1);
        cpu.push((0xc, 5));
        cpu.registers.set_bc(x1);
        cpu.push((0xc, 5));
        cpu.registers.set_bc(y2);
        cpu.push((0xc, 5));
        cpu.registers.set_bc(x2);
        cpu.push((0xc, 5));
        cpu.pop((0xd, 1));
        assert_eq!(cpu.registers.get_de(), x2);
        cpu.pop((0xd, 1));
        assert_eq!(cpu.registers.get_de(), y2);
        cpu.pop((0xd, 1));
        assert_eq!(cpu.registers.get_de(), x1);
        cpu.pop((0xd, 1));
        assert_eq!(cpu.registers.get_de(), y1);
    }
}