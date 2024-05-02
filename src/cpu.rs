use crate::bus::Bus;
use crate::register::Register;

pub struct Cpu {
    pub bus: Bus,
    a: Register,
    b: Register,
    c: Register,
    d: Register,
    e: Register,
    f: Register,
    h: Register,
    l: Register,
    sp: u16,
    pc: u16,
    counter: u32,
}

impl Cpu {
    // Constructor for Cpu
    pub fn new() -> Cpu {
        let bus = Bus::new();
        let mut cpu = Cpu { bus, a: Register { value: 0 }, b: Register { value: 0 }, c: Register { value: 0 }, d: Register { value: 0 }, e: Register { value: 0 }, f: Register { value: 0 }, h: Register { value: 0 }, l: Register { value: 0 }, pc: 0, sp: 0, counter: 0 };
        cpu.set_sp(crate::bus::HRAM_END);
        cpu.set_pc(0x0100);
        cpu.set_bc(0x0013);
        cpu.set_af(0x01B0);
        cpu.set_de(0x00D8);
        cpu.set_hl(0x014D);
        cpu
    }

    fn log(&self) {
        println!("A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: 00:{:04X} ({:02X} {:02X} {:02X} {:02X})",
                 self.a.get(),
                 self.f.get(),
                 self.b.get(),
                 self.c.get(),
                 self.d.get(),
                 self.e.get(),
                 self.h.get(),
                 self.l.get(),
                 self.get_sp(),
                 self.get_pc(),
                 self.bus.get(self.get_pc()),
                 self.bus.get(self.get_pc() + 1),
                 self.bus.get(self.get_pc() + 2),
                 self.bus.get(self.get_pc() + 3)
        )
    }
    pub fn step(&mut self) {
        let opcode = self.bus.get(self.get_pc());
        let inst = (opcode >> 4, opcode & 0xF);
        //println!("Executing {:#02x}({:#01x}, {:#01x}) at {:#02x}", opcode, inst.0, inst.1, self.get_pc());
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
            !self.prefix(inst) &&
            !self.push(inst) {
            panic!("Not implemented yet {:#02x} at {:#02x}", opcode, self.get_pc())
        }
        self.counter += 1;
        //if self.counter % 100 == 0 {
        //    println!("{}", self.counter);
        //self.log();
        //}
    }

    fn _cycles(&self, _count: usize) {}
    fn _pc(&mut self, count: u16) { self.set_pc(self.get_pc() + count) }

    fn get_flag_c(&self) -> bool {
        self.f.get_bit(4)
    }
    fn get_flag_h(&self) -> bool {
        self.f.get_bit(5)
    }
    fn get_flag_n(&self) -> bool {
        self.f.get_bit(6)
    }
    fn get_flag_z(&self) -> bool {
        self.f.get_bit(7)
    }
    fn set_flag_c(&mut self, value: bool) {
        self.f.set_bit(4, value)
    }
    fn set_flag_h(&mut self, value: bool) {
        self.f.set_bit(5, value)
    }
    fn set_flag_n(&mut self, value: bool) {
        self.f.set_bit(6, value)
    }
    fn set_flag_z(&mut self, value: bool) {
        self.f.set_bit(7, value)
    }
    fn get_af(&self) -> u16 {
        ((self.a.get() as u16) << 8) | self.f.get() as u16
    }
    fn get_bc(&self) -> u16 {
        ((self.b.get() as u16) << 8) | self.c.get() as u16
    }
    fn get_de(&self) -> u16 {
        ((self.d.get() as u16) << 8) | self.e.get() as u16
    }
    fn get_hl(&self) -> u16 {
        ((self.h.get() as u16) << 8) | self.l.get() as u16
    }
    fn get_sp(&self) -> u16 {
        self.sp
    }
    fn get_pc(&self) -> u16 {
        self.pc
    }
    fn set_sp(&mut self, value: u16) {
        self.sp = value
    }
    fn set_pc(&mut self, value: u16) {
        self.pc = value
    }

    fn set_af(&mut self, value: u16) {
        self.a.set((value >> 8) as u8);
        self.f.set(value as u8)
    }
    fn set_bc(&mut self, value: u16) {
        self.b.set((value >> 8) as u8);
        self.c.set(value as u8)
    }
    fn set_de(&mut self, value: u16) {
        self.d.set((value >> 8) as u8);
        self.e.set(value as u8)
    }
    fn set_hl(&mut self, value: u16) {
        self.h.set((value >> 8) as u8);
        self.l.set(value as u8)
    }
    fn arithmetic_flags16(&self, value: u16, func: fn(u16, u16) -> (u16, bool)) -> (u16, bool, bool) {
        let (val, c) = func(self.get_hl(), value);
        let (_, h) = func(self.get_hl() << 8, value << 8);
        (val, c, h)
    }
    fn add16(&mut self, value: u16) {
        let (val, c, h) = self.arithmetic_flags16(value, u16::overflowing_add);
        self.set_hl(val);
        self.set_flag_n(false);
        self.set_flag_c(c);
        self.set_flag_h(h);
    }
    fn inc16_bc(&mut self) { self.set_bc(self.get_bc().wrapping_add(1)); }
    fn dec16_bc(&mut self) { self.set_bc(self.get_bc().wrapping_sub(1)); }
    fn inc16_de(&mut self) { self.set_de(self.get_de().wrapping_add(1)); }
    fn dec16_de(&mut self) { self.set_de(self.get_de().wrapping_sub(1)); }
    fn inc16_hl(&mut self) { self.set_hl(self.get_hl().wrapping_add(1)); }
    fn dec16_hl(&mut self) { self.set_hl(self.get_hl().wrapping_sub(1)); }
    fn inc16_sp(&mut self) { self.set_sp(self.get_sp().wrapping_add(1)); }
    fn dec16_sp(&mut self) { self.set_sp(self.get_sp().wrapping_sub(1)); }
    pub fn inc_m(&mut self, address: u16) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.a.arithmetic_flags(self.bus.get(address), 1, u8::overflowing_add, false, self.get_flag_c());
        self.bus.set(address, val);
        (val == 0, false, h, c)
    }
    pub fn dec_m(&mut self, address: u16) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.a.arithmetic_flags(self.bus.get(address), 1, u8::overflowing_sub, false, self.get_flag_c());
        self.bus.set(address, val);
        (val == 0, true, h, c)
    }
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
        if inst != (0xc, 0xb) {
            return false;
        }
        let opcode = self.bus.get(self.get_pc() + 1);
        let inst = (opcode >> 4, opcode & 0xF);
        let current_flag = self.get_flag_c();
        let (z, n, h, c);
        if inst.1 == 6 || inst.1 == 0xe {
            let address = self.get_hl();
            let func = match inst {
                (0, 6) => Bus::rlc,
                (0, 0xe) => Bus::rrc,
                (1, 6) => Bus::rl,
                (1, 0xe) => Bus::rr,
                (2, 6) => Bus::sla,
                (2, 0xe) => Bus::sra,
                (3, 6) => Bus::swap,
                (3, 0xe) => Bus::srl,
                (4..=7, 0xe | 6) => Bus::bit,
                (8..=0xb, 0xe | 6) => Bus::reset,
                (0xc..=0xf, 0xe | 6) => Bus::setb,
                _ => panic!("Should be impossible!")
            };
            (z, n, h, c) = match inst {
                (4 | 8 | 0xc, 6) => func(&mut self.bus, false, false, 0, address),
                (4 | 8 | 0xc, 0xe) => func(&mut self.bus, false, false, 1, address),
                (5 | 9 | 0xd, 6) => func(&mut self.bus, false, false, 2, address),
                (5 | 9 | 0xd, 0xe) => func(&mut self.bus, false, false, 3, address),
                (6 | 0xa | 0xe, 6) => func(&mut self.bus, false, false, 4, address),
                (6 | 0xa | 0xe, 0xe) => func(&mut self.bus, false, false, 5, address),
                (7 | 0xb | 0xf, 6) => func(&mut self.bus, false, false, 6, address),
                (7 | 0xb | 0xf, 0xe) => func(&mut self.bus, false, false, 7, address),
                _ => func(&mut self.bus, false, current_flag, 0, 0)
            };
            match inst.0 {
                4..=7 => { self._cycles(1) }
                _ => self._cycles(2)
            }
        } else {
            let func = match inst {
                (0, 0..=5 | 7) => Register::rlc,
                (0, 8..=0xc | 0xf) => Register::rrc,
                (1, 0..=5 | 7) => Register::rl,
                (1, 8..=0xc | 0xf) => Register::rr,
                (2, 0..=5 | 7) => Register::sla,
                (2, 8..=0xc | 0xf) => Register::sra,
                (3, 0..=5 | 7) => Register::swap,
                (3, 8..=0xc | 0xf) => Register::srl,
                (4..=7, 0..=0x5 | 0x7..=0xc | 0xf) => Register::bit,
                (8..=0xb, 0..=0x5 | 0x7..=0xc | 0xf) => Register::reset,
                (0xc..=0xf, 0..=0x5 | 0x7..=0xc | 0xf) => Register::setb,
                _ => panic!("Should be impossible!")
            };
            let bit = match inst {
                (4 | 8 | 0xc, ..=7) => 0,
                (4 | 8 | 0xc, 8..) => 1,
                (5 | 9 | 0xd, ..=7) => 2,
                (5 | 9 | 0xd, 8..) => 3,
                (6 | 0xa | 0xe, ..=7) => 4,
                (6 | 0xa | 0xe, 8..) => 5,
                (7 | 0xb | 0xf, ..=7) => 6,
                (7 | 0xb | 0xf, 8..) => 7,
                _ => 0
            };
            (z, n, h, c) = match inst {
                (0 | 0x8, _) => func(&mut self.b, false, current_flag, bit, 0),
                (1 | 0x9, _) => func(&mut self.c, false, current_flag, bit, 0),
                (2 | 0xa, _) => func(&mut self.d, false, current_flag, bit, 0),
                (3 | 0xb, _) => func(&mut self.e, false, current_flag, bit, 0),
                (4 | 0xc, _) => func(&mut self.h, false, current_flag, bit, 0),
                (5 | 0xd, _) => func(&mut self.l, false, current_flag, bit, 0),
                (7 | 0xf, _) => func(&mut self.a, false, current_flag, bit, 0),
                _ => panic!("Unknown register")
            };
        }
        match inst.0 {
            ..=3 => {
                self.set_flag_z(z);
                self.set_flag_n(n);
                self.set_flag_h(h);
                self.set_flag_c(c);
            }
            4..=7 => {
                self.set_flag_z(z);
                self.set_flag_n(n);
                self.set_flag_h(h);
            }
            _ => {}
        }
        self._pc(2);
        true
    }
    fn call(&mut self, inst: (u8, u8)) -> bool {
        let new_pc = match inst {
            (0xc..=0xd, 4 | 0xc) | (0xc, 0xd) => self.bus.get16(self.get_pc() + 1),
            _ => return false
        };
        let condition = match inst {
            (0xc, 4) => !self.get_flag_z(),
            (0xd, 4) => !self.get_flag_c(),
            (0xc, 0xc) => self.get_flag_z(),
            (0xd, 0xc) => self.get_flag_c(),
            _ => true
        };
        self._pc(3);
        if condition {
            self._push(self.get_pc());
            self.set_pc(new_pc);
            self._cycles(3);
        }
        self._cycles(3);
        true
    }
    fn ret(&mut self, inst: (u8, u8)) -> bool {
        let condition = match inst {
            (0xc, 0) => !self.get_flag_z(),
            (0xd, 0) => !self.get_flag_c(),
            (0xc, 0x8) => self.get_flag_z(),
            (0xd, 0x8) => self.get_flag_c(),
            (0xc, 0x9) => true,
            _ => return false
        };
        self._pc(1);
        if condition {
            let new_pc = self._pop();
            self.set_pc(new_pc);
            if inst == (0xc, 0x9) {
                self._cycles(4);
            } else {
                self._cycles(5);
            }
        } else {
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
        self._push(self.get_pc());
        self.set_pc(new_pc);
        true
    }
    fn rotate(&mut self, inst: (u8, u8)) -> bool {
        let (z, n, h, c) = match inst {
            (0, 7) => self.a.rotate_left_a(true, false),
            (1, 7) => self.a.rotate_left_a(false, self.get_flag_c()),
            (0, 0xf) => self.a.rotate_right_a(true, false),
            (1, 0xf) => self.a.rotate_right_a(false, self.get_flag_c()),
            _ => return false
        };
        self.set_flag_c(c);
        self.set_flag_n(n);
        self.set_flag_h(h);
        self.set_flag_z(z);
        self._cycles(1);
        self._pc(1);
        true
    }
    fn _pop(&mut self) -> u16 {
        let val = self.bus.get16(self.get_sp());
        self.inc16_sp();
        self.inc16_sp();
        val
    }
    fn pop(&mut self, inst: (u8, u8)) -> bool {
        let val = match inst {
            (0xc..=0xf, 1) => self._pop(),
            _ => return false
        };

        match inst {
            (0xc, 1) => self.set_bc(val),
            (0xd, 1) => self.set_de(val),
            (0xe, 1) => self.set_hl(val),
            (0xf, 1) => self.set_af(val),
            _ => return false
        }
        self._cycles(3);
        self._pc(1);
        true
    }
    fn _push(&mut self, value: u16) {
        self.bus.set16(self.get_sp() - 2, value);
        self.dec16_sp();
        self.dec16_sp();
    }
    fn push(&mut self, inst: (u8, u8)) -> bool {
        let value = match inst {
            (0xc, 5) => self.get_bc(),
            (0xd, 5) => self.get_de(),
            (0xe, 5) => self.get_hl(),
            (0xf, 5) => self.get_af(),
            _ => return false
        };
        self._push(value);

        self._cycles(4);
        self._pc(1);
        true
    }
    fn load16(&mut self, inst: (u8, u8)) -> bool {
        let val: u16 = match inst {
            (0..=3, 1) => self.bus.get16(self.get_pc() + 1),
            (0, 8) => self.get_sp(),
            (0xf, 9) => self.get_hl(),
            _ => return false
        };
        match inst {
            (0, 1) => self.set_bc(val),
            (1, 1) => self.set_de(val),
            (2, 1) => self.set_hl(val),
            (3, 1) => self.set_sp(val),
            (0, 8) => self.bus.set16(self.bus.get16(self.get_pc() + 1), val),
            (0xf, 9) => self.set_sp(val),
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
            (4..=7, 0x0 | 0x8) => self.b.get(),
            (4..=7, 0x1 | 0x9) => self.c.get(),
            (4..=7, 0x2 | 0xa) => self.d.get(),
            (4..=7, 0x3 | 0xb) => self.e.get(),
            (4..=7, 0x4 | 0xc) => self.h.get(),
            (4..=7, 0x5 | 0xd) => self.l.get(),
            (4..=7, 0x6 | 0xe) | (2..=3, 0xa) => self.bus.get(self.get_hl()),
            (4..=7, 0x7 | 0xf) | (0..=3, 0x2) | (0xe, 0 | 2 | 0xa) => self.a.get(),
            (0x0, 0xa) => self.bus.get(self.get_bc()),
            (0x1, 0xa) => self.bus.get(self.get_de()),
            (0..=3, 0x6 | 0xe) => self.bus.get(self.get_pc() + 1),
            (0xf, 0xa) => self.bus.get(self.bus.get16(self.get_pc() + 1)),
            (0xf, 0) => self.bus.get(self.bus.get(self.get_pc() + 1) as u16 + 0xFF00),
            (0xf, 2) => self.bus.get(self.c.get() as u16 + 0xFF00),
            _ => return false
        };
        match inst {
            (0x4, ..=7) | (0, 0x6) => self.b.set(val),
            (0x4, 8..) | (0, 0xe) => self.c.set(val),
            (0x5, ..=7) | (1, 0x6) => self.d.set(val),
            (0x5, 8..) | (1, 0xe) => self.e.set(val),
            (0x6, ..=7) | (2, 0x6) => self.h.set(val),
            (0x6, 8..) | (2, 0xe) => self.l.set(val),
            (0x7, ..=5 | 7) | (2..=3, 0x2) | (3, 0x6) => self.bus.set(self.get_hl(), val),
            (0x7, 8..) | (0..=3, 0xa) | (3, 0xe) | (0xf, 0 | 2 | 0xa) => self.a.set(val),
            (0x0, 0x2) => self.bus.set(self.get_bc(), val),
            (0x1, 0x2) => self.bus.set(self.get_de(), val),
            (0xe, 0xa) => self.bus.set(self.bus.get16(self.get_pc() + 1), val),
            (0xe, 0) => self.bus.set(self.bus.get(self.get_pc() + 1) as u16 + 0xFF00, val),
            (0xe, 2) => self.bus.set(self.c.get() as u16 + 0xFF00, val),
            _ => return false
        };
        match inst {
            (2, 2 | 0xa) => self.set_hl(self.get_hl() + 1),
            (3, 2 | 0xa) => self.set_hl(self.get_hl() - 1),
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
            (0, 9) => self.get_bc(),
            (1, 9) => self.get_de(),
            (2, 9) => self.get_hl(),
            (3, 9) => self.get_sp(),
            _ => 0
        };
        match inst {
            (0, 3) => self.inc16_bc(),
            (1, 3) => self.inc16_de(),
            (2, 3) => self.inc16_hl(),
            (3, 3) => self.inc16_sp(),
            (0, 0xb) => self.dec16_bc(),
            (1, 0xb) => self.dec16_de(),
            (2, 0xb) => self.dec16_hl(),
            (3, 0xb) => self.dec16_sp(),
            (0..=3, 9) => self.add16(val),
            _ => return false
        };
        self._cycles(2);
        self._pc(1);
        true
    }
    fn alu(&mut self, inst: (u8, u8)) -> bool {
        let old_carry_flag = self.get_flag_c();
        let val = match inst {
            (8..=0xb, 0 | 0x8) => self.b.get(),
            (8..=0xb, 1 | 0x9) => self.c.get(),
            (8..=0xb, 2 | 0xa) => self.d.get(),
            (8..=0xb, 3 | 0xb) => self.e.get(),
            (8..=0xb, 4 | 0xc) => self.h.get(),
            (8..=0xb, 5 | 0xd) => self.l.get(),
            (8..=0xb, 6 | 0xe) => self.bus.get(self.get_hl()),
            (8..=0xb, 7 | 0xf) => self.a.get(),
            (0..=3, 4 | 5 | 0xc | 0xd) => 0,
            (0xc..=0xf, 6 | 0xe) => self.bus.get(self.get_pc() + 1),
            _ => return false
        };
        let (z, n, h, c) = match inst {
            (0x8, 0..=0x7) | (0xc, 6) => self.a.add(val, false, false),
            (0x8, 8..=0xf) | (0xc, 0xe) => self.a.add(val, true, self.get_flag_c()),
            (0x9, 0..=0x7) | (0xd, 6) => self.a.sub(val, false, false),
            (0x9, 8..=0xf) | (0xd, 0xe) => self.a.sub(val, true, self.get_flag_c()),
            (0xa, 0..=0x7) | (0xe, 6) => self.a.and(val),
            (0xa, 8..=0xf) | (0xe, 0xe) => self.a.xor(val),
            (0xb, 0..=0x7) | (0xf, 6) => self.a.or(val),
            (0xb, 8..=0xf) | (0xf, 0xe) => self.a.cp(val),
            (0, 4) => self.b.inc(),
            (0, 5) => self.b.dec(),
            (1, 4) => self.d.inc(),
            (1, 5) => self.d.dec(),
            (2, 4) => self.h.inc(),
            (2, 5) => self.h.dec(),
            (3, 4) => self.inc_m(self.get_hl()),
            (3, 5) => self.dec_m(self.get_hl()),
            (0, 0xc) => self.c.inc(),
            (0, 0xd) => self.c.dec(),
            (1, 0xc) => self.e.inc(),
            (1, 0xd) => self.e.dec(),
            (2, 0xc) => self.l.inc(),
            (2, 0xd) => self.l.dec(),
            (3, 0xc) => self.a.inc(),
            (3, 0xd) => self.a.dec(),
            _ => return false
        };
        self.set_flag_z(z);
        self.set_flag_n(n);
        self.set_flag_h(h);
        self.set_flag_c(c);
        if inst.0 <= 3 {
            self.set_flag_c(old_carry_flag);
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
            (2, 0) | (0xc, 2) => !self.get_flag_z(),
            (2, 8) | (0xc, 0xa) => self.get_flag_z(),
            (3, 0) | (0xd, 2) => !self.get_flag_c(),
            (3, 8) | (0xd, 0xa) => self.get_flag_c(),
            _ => true
        };
        let pc = self.get_pc();
        if condition {
            match inst {
                (2..=3, 0) | (1..=3, 8) => {
                    self.set_pc(pc.wrapping_add_signed(self.bus.gets(pc + 1) as i16) + 2);
                    self._cycles(3);
                },
                (0xc..=0xd, 2) | (0xc..=0xd, 0xa) | (0xc, 0x3) => {
                    self.set_pc(self.bus.get16(pc + 1));
                    self._cycles(1)
                },
                (0xe, 0x9) => {
                    self.set_pc(self.get_hl());
                    self._cycles(1)
                },
                _ => return false
            }
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
        cpu.bus.set(cpu.get_pc(), 0xe8);
        cpu.step();
    }

    #[test]
    fn jump() {
        let mut cpu = Cpu::new();
        let mut rng = rand::thread_rng();


        for _ in [..=30] {
            let x1 = rng.gen_range(0..255);
            let y1 = rng.gen_range(0..0xFF00);
            cpu.set_pc(y1);
            cpu.bus.set(y1 + 1, x1);

            cpu.jump((0x1, 0x8));
            assert_eq!(cpu.get_pc(), y1.wrapping_add_signed((x1 as i8) as i16) + 2);
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

        cpu.c.set(x1);
        cpu.d.set(x2);

        cpu.load((0x5, 0x1));
        assert_eq!(cpu.c.get(), x1);
        assert_eq!(cpu.d.get(), x1);

        cpu.set_hl(x5);
        cpu.a.set(x3);

        cpu.load((0x2, 0x2));
        assert_eq!(cpu.bus.get(x5), x3);

        cpu.b.set(x4);
        cpu.load((0x7, 0x0));
        assert_eq!(cpu.bus.get(x5 + 1), x4);

        cpu.set_bc(x5 + 1);
        cpu.load((0x0, 0xa));
        assert_eq!(cpu.a.get(), x4);

        cpu.bus.set(cpu.get_pc() + 1, x1);
        cpu.load((1, 0xe));
        assert_eq!(cpu.e.get(), x1);
    }

    #[test]
    fn rotate_a() {
        let mut cpu = Cpu::new();

        let x1 = 0b01010101;
        let x2 = 0b10101010;

        cpu.a.set(x1);
        cpu.rotate((0, 7));
        assert_eq!(cpu.a.get(), x2);
        assert_eq!(cpu.get_flag_c(), false);

        cpu.a.set(x1);
        cpu.rotate((0, 0xf));
        assert_eq!(cpu.a.get(), x2);
        assert_eq!(cpu.get_flag_c(), true);

        cpu.a.set(x2);
        cpu.rotate((0, 7));
        assert_eq!(cpu.a.get(), x1);
        assert_eq!(cpu.get_flag_c(), true);

        cpu.a.set(x2);
        cpu.rotate((0, 0xf));
        assert_eq!(cpu.a.get(), x1);
        assert_eq!(cpu.get_flag_c(), false);
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

        cpu.a.set(x1);
        cpu.d.set(x2);
        cpu.set_hl(y1);
        cpu.bus.set(y1, x3);

        cpu.alu((0x8, 0x2));
        assert_eq!(cpu.a.get(), x1.wrapping_add(x2));

        cpu.a.set(x1);
        cpu.alu((0x8, 0x6));
        assert_eq!(cpu.a.get(), x1.wrapping_add(x3));

        cpu.a.set(x1);
        cpu.alu((0x9, 0x2));
        assert_eq!(cpu.a.get(), x1.wrapping_sub(x2));

        cpu.a.set(x1);
        cpu.alu((0x9, 0x6));
        assert_eq!(cpu.a.get(), x1.wrapping_sub(x3));

        cpu.a.set(x1);
        cpu.set_flag_c(false);
        cpu.alu((0x8, 0xa));
        assert_eq!(cpu.a.get(), x1.wrapping_add(x2));

        cpu.a.set(x1);
        cpu.set_flag_c(true);
        cpu.alu((0x8, 0xe));
        assert_eq!(cpu.a.get(), x1.wrapping_add(x3 + 1));

        cpu.a.set(x1);
        cpu.set_flag_c(false);
        cpu.alu((0x9, 0xa));
        assert_eq!(cpu.a.get(), x1.wrapping_sub(x2));

        cpu.a.set(x1);
        cpu.set_flag_c(true);
        cpu.alu((0x9, 0xe));
        assert_eq!(cpu.a.get(), x1.wrapping_sub(x3 + 1));

        cpu.a.set(x1);
        cpu.alu((0xa, 0x2));
        assert_eq!(cpu.a.get(), x1 & x2);

        cpu.a.set(x1);
        cpu.alu((0xa, 0x6));
        assert_eq!(cpu.a.get(), x1 & x3);

        cpu.a.set(x1);
        cpu.alu((0xb, 0x2));
        assert_eq!(cpu.a.get(), x1 | x2);

        cpu.a.set(x1);
        cpu.alu((0xb, 0x6));
        assert_eq!(cpu.a.get(), x1 | x3);

        cpu.a.set(x1);
        cpu.alu((0xa, 0xa));
        assert_eq!(cpu.a.get(), x1 ^ x2);

        cpu.a.set(x1);
        cpu.alu((0xa, 0xe));
        assert_eq!(cpu.a.get(), x1 ^ x3);

        cpu.a.set(x4);
        cpu.b.set(x5);
        cpu.alu((0x9, 0));
        assert_eq!(cpu.get_flag_c(), true);
        assert_eq!(cpu.get_flag_n(), true);
        assert_eq!(cpu.get_flag_z(), x4 == x5);

        cpu.a.set(x4);
        cpu.b.set(x4);
        cpu.alu((0xb, 0x8));
        assert_eq!(cpu.get_flag_n(), true);
        assert_eq!(cpu.get_flag_z(), true);

        cpu.a.set(s2);
        cpu.b.set(s2);
        cpu.alu((0x8, 0));
        assert_eq!(cpu.get_flag_n(), false);
        assert_eq!(cpu.get_flag_h(), true);
    }

    #[test]
    fn inc() {
        let mut cpu = Cpu::new();

        let x1 = 0x1;
        let x2 = 0xff;
        let x3 = 0x0f;

        cpu.c.set(x1);
        cpu.alu((0, 0xc));
        assert_eq!(cpu.get_flag_z(), false);
        assert_eq!(cpu.get_flag_n(), false);

        cpu.c.set(x2);
        cpu.alu((0, 0xc));
        assert_eq!(cpu.get_flag_z(), true);
        assert_eq!(cpu.get_flag_n(), false);

        cpu.c.set(x3);
        cpu.alu((0, 0xc));
        assert_eq!(cpu.get_flag_h(), true);
        assert_eq!(cpu.get_flag_n(), false);
    }

    #[test]
    fn dec() {
        let mut cpu = Cpu::new();

        let x1 = 0x1;
        let x2 = 0xff;
        let x3 = 0x00;

        cpu.c.set(x2);
        cpu.alu((0, 0xd));
        assert_eq!(cpu.get_flag_z(), false);
        assert_eq!(cpu.get_flag_n(), true);

        cpu.c.set(x1);
        cpu.alu((0, 0xd));
        assert_eq!(cpu.get_flag_z(), true);
        assert_eq!(cpu.get_flag_n(), true);

        cpu.c.set(x3);
        cpu.alu((0, 0xd));
        assert_eq!(cpu.get_flag_h(), true);
        assert_eq!(cpu.get_flag_n(), true);
    }

    #[test]
    fn stack() {
        let mut cpu = Cpu::new();
        let mut rng = rand::thread_rng();

        let x1 = rng.gen_range(0..=255);
        let x2 = rng.gen_range(0..=255);
        let y1 = rng.gen_range(0..=0xFFFF);
        let y2 = rng.gen_range(0..=0xFFFF);

        cpu.set_bc(y1);
        cpu.push((0xc, 5));
        cpu.pop((0xd, 1));
        assert_eq!(cpu.get_de(), y1);

        cpu.set_bc(y1);
        cpu.push((0xc, 5));
        cpu.set_bc(x1);
        cpu.push((0xc, 5));
        cpu.set_bc(y2);
        cpu.push((0xc, 5));
        cpu.set_bc(x2);
        cpu.push((0xc, 5));
        cpu.pop((0xd, 1));
        assert_eq!(cpu.get_de(), x2);
        cpu.pop((0xd, 1));
        assert_eq!(cpu.get_de(), y2);
        cpu.pop((0xd, 1));
        assert_eq!(cpu.get_de(), x1);
        cpu.pop((0xd, 1));
        assert_eq!(cpu.get_de(), y1);
    }
}