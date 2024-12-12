use crate::bus::{Bus, INT_ENABLE, INT_REQUEST};
use crate::register::Register;

pub struct Cpu {
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
    counter: usize,
    ime: bool,
    halted: bool,
}

impl Cpu {
    // Constructor for Cpu
    pub fn new() -> Cpu {
        let mut cpu = Cpu { a: Register { value: 0 }, b: Register { value: 0 }, c: Register { value: 0 }, d: Register { value: 0 }, e: Register { value: 0 }, f: Register { value: 0 }, h: Register { value: 0 }, l: Register { value: 0 }, pc: 0, sp: 0, counter: 0, ime: false, halted: false };
        cpu.set_sp(crate::bus::HRAM_END);
        cpu.set_pc(0x0100);
        cpu.set_bc(0x0013);
        cpu.set_af(0x01B0);
        cpu.set_de(0x00D8);
        cpu.set_hl(0x014D);
        cpu
    }

    fn log(&self, bus: &Bus) {
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
                 bus.get(self.get_pc()),
                 bus.get(self.get_pc() + 1),
                 bus.get(self.get_pc() + 2),
                 bus.get(self.get_pc() + 3)
        )
    }
    pub fn step(&mut self, mut bus: &mut Bus, log: bool) -> usize {
        if self.halted{
            if bus.get(INT_ENABLE) & bus.get(INT_REQUEST) > 0{
                self.halted = false;
            } else{
                return 1;
            }
        }
        let opcode = bus.get(self.get_pc());
        let inst = (opcode >> 4, opcode & 0xF);
        let cycles = self.counter;
        //println!("Executing {:#02x}({:#01x}, {:#01x}) at {:#02x}", opcode, inst.0, inst.1, self.get_pc());
        if !self.load(inst, &mut bus) &&
            !self.alu(inst, &mut bus) &&
            !self.load16(inst, &mut bus) &&
            !self.alu16(inst, &mut bus) &&
            !self.jump(inst, &mut bus) &&
            !self.pop(inst, &mut bus) &&
            !self.misc(inst, &mut bus) &&
            !self.rotate(inst, &mut bus) &&
            !self.reset(inst, &mut bus) &&
            !self.call(inst, &mut bus) &&
            !self.ret(inst, &mut bus) &&
            !self.prefix(inst, &mut bus) &&
            !self.push(inst, &mut bus) {
            panic!("Not implemented yet {:#02x} at {:#02x}", opcode, self.get_pc())
        }
        //if log { self.log(&bus) };
        //if self.counter % 100 == 0 {
        //    println!("{}", self.counter);
        //}
        //self.counter = 0;
        self.counter - cycles
    }

    fn _cycles(&mut self, _count: usize) {
        self.counter += _count
    }
    fn _pc(&mut self, count: u16) { self.set_pc(self.get_pc() + count) }
    pub fn interrupt(&mut self, mut bus: &mut Bus, address: u16) -> usize {
        self._cycles(2);
        self._push(self.get_pc(), bus);
        self._cycles(2);
        self.set_pc(address);
        self._cycles(1);
        self.halted = false;
        self.set_ime(false);
        5 + self.step(&mut bus, true)
    }
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
        self.f.set(value as u8 & 0xF0)
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
    fn arithmetic_flags16(&self, base: u16, value: u16, func: fn(u16, u16) -> (u16, bool)) -> (u16, bool, bool) {
        let (val, c) = func(self.get_hl(), value);
        let (_, h) = func(base << 4, value << 4);
        (val, c, h)
    }
    fn add16(&mut self, value: u16) {
        let (val, c, h) = self.arithmetic_flags16(self.get_hl(), value, u16::overflowing_add);
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
    fn inc_m(&mut self, address: u16, bus: &mut Bus) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.a.arithmetic_flags(bus.get(address), 1, u8::overflowing_add, false, self.get_flag_c());
        bus.set(address, val);
        (val == 0, false, h, c)
    }
    fn dec_m(&mut self, address: u16, bus: &mut Bus) -> (bool, bool, bool, bool) {
        let (val, c, h) = self.a.arithmetic_flags(bus.get(address), 1, u8::overflowing_sub, false, self.get_flag_c());
        bus.set(address, val);
        (val == 0, true, h, c)
    }
    pub fn set_ime(&mut self, value: bool) {
        self.ime = value
    }
    pub fn get_ime(&mut self) -> bool {
        self.ime
    }
    fn misc(&mut self, inst: (u8, u8), mut bus: &mut Bus) -> bool {
        match inst {
            (0, 0) => {}
            (1, 0) => { self._pc(1) },
            (7, 6) => self.halted = true,
            (0xf, 3) => self.set_ime(false),
            (0xf, 0xb) => self.set_ime(true),
            (0x3, 0x7) => {
                self.set_flag_c(true);
                self.set_flag_n(false);
                self.set_flag_h(false);
            }
            (0x2, 0xf) => {
                self.a.set(self.a.get() ^ 0xFF);
                self.set_flag_n(true);
                self.set_flag_h(true);
            }
            (0x3, 0xf) => {
                self.set_flag_c(!self.get_flag_c());
                self.set_flag_n(false);
                self.set_flag_h(false);
            }
            (0xe, 0x8) => {
                let (val, _) = self.get_sp().overflowing_add_signed(bus.gets(self.get_pc() + 1) as i16);
                let (_, c) = (self.get_sp() << 8).overflowing_add((bus.get(self.get_pc() + 1) as u16) << 8);
                let (_, h) = (self.get_sp() << 12).overflowing_add((bus.get(self.get_pc() + 1) as u16) << 12);
                self.set_sp(val);
                self.set_flag_c(c);
                self.set_flag_h(h);
                self.set_flag_z(false);
                self.set_flag_n(false);
                self._cycles(3);
                self._pc(1);
            }
            (0xf, 0x8) => {
                let (val, _) = self.get_sp().overflowing_add_signed(bus.gets(self.get_pc() + 1) as i16);
                let (_, c) = (self.get_sp() << 8).overflowing_add((bus.get(self.get_pc() + 1) as u16) << 8);
                let (_, h) = (self.get_sp() << 12).overflowing_add((bus.get(self.get_pc() + 1) as u16) << 12);
                self.set_hl(val);
                self.set_flag_c(c);
                self.set_flag_h(h);
                self.set_flag_z(false);
                self.set_flag_n(false);
                self._cycles(2);
                self._pc(1);
            }
            (0x2, 0x7) => {
                if !self.get_flag_n() {  // after an addition, adjust if (half-)carry occurred or if result is out of bounds
                    if self.get_flag_c() || self.a.get() > 0x99 {
                        self.a.add(0x60, false, false);
                        self.set_flag_c(true);
                    }
                    if self.get_flag_h() || (self.a.get() & 0x0f) > 0x09 { self.a.add(0x6, false, false); }
                } else {  // after a subtraction, only adjust if (half-)carry occurred
                    if self.get_flag_c() { self.a.sub(0x60, false, false); }
                    if self.get_flag_h() { self.a.sub(0x6, false, false); }
                }
                // these flags are always updated
                self.set_flag_z(self.a.get() == 0); // the usual z flag
                self.set_flag_h(false);
            }
            (0xd, 0x9) => {
                self.set_ime(true);
                self.ret((0xc, 0x9), &mut bus);
                return true;
            }
            _ => return false
        }
        self._cycles(1);
        self._pc(1);
        true
    }
    fn prefix(&mut self, inst: (u8, u8), mut bus: &mut Bus) -> bool {
        if inst != (0xc, 0xb) {
            return false;
        }
        let opcode = bus.get(self.get_pc() + 1);
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
                (4 | 8 | 0xc, 6) => func(&mut bus, false, false, 0, address),
                (4 | 8 | 0xc, 0xe) => func(&mut bus, false, false, 1, address),
                (5 | 9 | 0xd, 6) => func(&mut bus, false, false, 2, address),
                (5 | 9 | 0xd, 0xe) => func(&mut bus, false, false, 3, address),
                (6 | 0xa | 0xe, 6) => func(&mut bus, false, false, 4, address),
                (6 | 0xa | 0xe, 0xe) => func(&mut bus, false, false, 5, address),
                (7 | 0xb | 0xf, 6) => func(&mut bus, false, false, 6, address),
                (7 | 0xb | 0xf, 0xe) => func(&mut bus, false, false, 7, address),
                _ => func(&mut bus, false, current_flag, 0, address)
            };
            match inst.0 {
                4..=7 => { self._cycles(3) }
                _ => self._cycles(4)
            }
        } else {
            let func = match inst {
                (0, 0..=5 | 7) => Register::rlc,
                (0, 8..=0xd | 0xf) => Register::rrc,
                (1, 0..=5 | 7) => Register::rl,
                (1, 8..=0xd | 0xf) => Register::rr,
                (2, 0..=5 | 7) => Register::sla,
                (2, 8..=0xd | 0xf) => Register::sra,
                (3, 0..=5 | 7) => Register::swap,
                (3, 8..=0xd | 0xf) => Register::srl,
                (4..=7, 0..=0x5 | 0x7..=0xd | 0xf) => Register::bit,
                (8..=0xb, 0..=0x5 | 0x7..=0xd | 0xf) => Register::reset,
                (0xc..=0xf, 0..=0x5 | 0x7..=0xd | 0xf) => Register::setb,
                _ => panic!("Should be impossible! {:#02x}", opcode)
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
                (_, 0 | 0x8) => func(&mut self.b, false, current_flag, bit, 0),
                (_, 1 | 0x9) => func(&mut self.c, false, current_flag, bit, 0),
                (_, 2 | 0xa) => func(&mut self.d, false, current_flag, bit, 0),
                (_, 3 | 0xb) => func(&mut self.e, false, current_flag, bit, 0),
                (_, 4 | 0xc) => func(&mut self.h, false, current_flag, bit, 0),
                (_, 5 | 0xd) => func(&mut self.l, false, current_flag, bit, 0),
                (_, 7 | 0xf) => func(&mut self.a, false, current_flag, bit, 0),
                _ => panic!("Unknown register")
            };
            self._cycles(2)
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
    fn call(&mut self, inst: (u8, u8), mut bus: &mut Bus) -> bool {
        let new_pc = match inst {
            (0xc..=0xd, 4 | 0xc) | (0xc, 0xd) => bus.get16(self.get_pc() + 1),
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
            self._push(self.get_pc(), &mut bus);
            self.set_pc(new_pc);
            self._cycles(3);
        }
        self._cycles(3);
        true
    }
    fn ret(&mut self, inst: (u8, u8), mut bus: &mut Bus) -> bool {
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
            let new_pc = self._pop(&mut bus);
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
    fn reset(&mut self, inst: (u8, u8), mut bus: &mut Bus) -> bool {
        let new_pc = match inst {
            (0xc, 7) => 0x08,
            (0xc, 0xf) => 0x08,
            (0xd, 7) => 0x10,
            (0xd, 0xf) => 0x18,
            (0xe, 7) => 0x20,
            (0xe, 0xf) => 0x28,
            (0xf, 7) => 0x30,
            (0xf, 0xf) => 0x38,
            _ => return false
        };
        self._pc(1);
        self._cycles(4);
        self._push(self.get_pc(), &mut bus);
        self.set_pc(new_pc);
        true
    }
    fn rotate(&mut self, inst: (u8, u8), _: &mut Bus) -> bool {
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
    fn _pop(&mut self, bus: &Bus) -> u16 {
        let val = bus.get16(self.get_sp());
        self.inc16_sp();
        self.inc16_sp();
        val
    }
    fn pop(&mut self, inst: (u8, u8), mut bus: &mut Bus) -> bool {
        let val = match inst {
            (0xc..=0xf, 1) => self._pop(&mut bus),
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
    fn _push(&mut self, value: u16, bus: &mut Bus) {
        bus.set16(self.get_sp() - 2, value);
        self.dec16_sp();
        self.dec16_sp();
    }
    fn push(&mut self, inst: (u8, u8), mut bus: &mut Bus) -> bool {
        let value = match inst {
            (0xc, 5) => self.get_bc(),
            (0xd, 5) => self.get_de(),
            (0xe, 5) => self.get_hl(),
            (0xf, 5) => self.get_af(),
            _ => return false
        };
        self._push(value, &mut bus);

        self._cycles(4);
        self._pc(1);
        true
    }
    fn load16(&mut self, inst: (u8, u8), bus: &mut Bus) -> bool {
        let val: u16 = match inst {
            (0..=3, 1) => bus.get16(self.get_pc() + 1),
            (0, 8) => self.get_sp(),
            (0xf, 9) => self.get_hl(),
            _ => return false
        };
        match inst {
            (0, 1) => self.set_bc(val),
            (1, 1) => self.set_de(val),
            (2, 1) => self.set_hl(val),
            (3, 1) | (0xf, 9) => self.set_sp(val),
            (0, 8) => bus.set16(bus.get16(self.get_pc() + 1), val),
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
    fn load(&mut self, inst: (u8, u8), bus: &mut Bus) -> bool {
        let val: u8 = match inst {
            (4..=7, 0x0 | 0x8) => self.b.get(),
            (4..=7, 0x1 | 0x9) => self.c.get(),
            (4..=7, 0x2 | 0xa) => self.d.get(),
            (4..=7, 0x3 | 0xb) => self.e.get(),
            (4..=7, 0x4 | 0xc) => self.h.get(),
            (4..=7, 0x5 | 0xd) => self.l.get(),
            (4..=7, 0x6 | 0xe) | (2..=3, 0xa) => bus.get(self.get_hl()),
            (4..=7, 0x7 | 0xf) | (0..=3, 0x2) | (0xe, 0 | 2 | 0xa) => self.a.get(),
            (0x0, 0xa) => bus.get(self.get_bc()),
            (0x1, 0xa) => bus.get(self.get_de()),
            (0..=3, 0x6 | 0xe) => bus.get(self.get_pc() + 1),
            (0xf, 0xa) => bus.get(bus.get16(self.get_pc() + 1)),
            (0xf, 0) => {
                bus.get(bus.get(self.get_pc() + 1) as u16 + 0xFF00)
            }
            (0xf, 2) => bus.get(self.c.get() as u16 + 0xFF00),
            _ => return false
        };
        match inst {
            (0x4, ..=7) | (0, 0x6) => self.b.set(val),
            (0x4, 8..) | (0, 0xe) => self.c.set(val),
            (0x5, ..=7) | (1, 0x6) => self.d.set(val),
            (0x5, 8..) | (1, 0xe) => self.e.set(val),
            (0x6, ..=7) | (2, 0x6) => self.h.set(val),
            (0x6, 8..) | (2, 0xe) => self.l.set(val),
            (0x7, ..=5 | 7) | (2..=3, 0x2) | (3, 0x6) => bus.set(self.get_hl(), val),
            (0x7, 8..) | (0..=3, 0xa) | (3, 0xe) | (0xf, 0 | 2 | 0xa) => self.a.set(val),
            (0x0, 0x2) => bus.set(self.get_bc(), val),
            (0x1, 0x2) => bus.set(self.get_de(), val),
            (0xe, 0xa) => bus.set(bus.get16(self.get_pc() + 1), val),
            (0xe, 0) => {
                    bus.set(bus.get(self.get_pc() + 1) as u16 + 0xFF00, val)
            },
            (0xe, 2) => bus.set(self.c.get() as u16 + 0xFF00, val),
            _ => return false
        };
        match inst {
            (2, 2 | 0xa) => self.set_hl(self.get_hl().wrapping_add(1)),
            (3, 2 | 0xa) => self.set_hl(self.get_hl().wrapping_sub(1)),
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
    fn alu16(&mut self, inst: (u8, u8), _: &mut Bus) -> bool {
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
    fn alu(&mut self, inst: (u8, u8), mut bus: &mut Bus) -> bool {
        let old_carry_flag = self.get_flag_c();
        let val = match inst {
            (8..=0xb, 0 | 0x8) => self.b.get(),
            (8..=0xb, 1 | 0x9) => self.c.get(),
            (8..=0xb, 2 | 0xa) => self.d.get(),
            (8..=0xb, 3 | 0xb) => self.e.get(),
            (8..=0xb, 4 | 0xc) => self.h.get(),
            (8..=0xb, 5 | 0xd) => self.l.get(),
            (8..=0xb, 6 | 0xe) => bus.get(self.get_hl()),
            (8..=0xb, 7 | 0xf) => self.a.get(),
            (0..=3, 4 | 5 | 0xc | 0xd) => 0,
            (0xc..=0xf, 6 | 0xe) => bus.get(self.get_pc() + 1),
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
            (3, 4) => self.inc_m(self.get_hl(), &mut bus),
            (3, 5) => self.dec_m(self.get_hl(), &mut bus),
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
            (3, 4 | 5) => {
                self._cycles(3);
                self._pc(1)
            }
            _ => {
                self._cycles(1);
                self._pc(1)
            }
        }
        true
    }
    fn jump(&mut self, inst: (u8, u8), bus: &mut Bus) -> bool {
        let condition = match inst {
            (2, 0) | (0xc, 2) => !self.get_flag_z(),
            (2, 8) | (0xc, 0xa) => self.get_flag_z(),
            (3, 0) | (0xd, 2) => !self.get_flag_c(),
            (3, 8) | (0xd, 0xa) => self.get_flag_c(),
            (1, 8) | (0xc, 3) | (0xe, 9) => true,
            _ => return false
        };
        let pc = self.get_pc();
        if condition {
            match inst {
                (2..=3, 0) | (1..=3, 8) => {
                    self.set_pc(pc.wrapping_add_signed(bus.gets(pc + 1) as i16) + 2);
                    self._cycles(3)
                }
                (0xc..=0xd, 2) | (0xc..=0xd, 0xa) | (0xc, 0x3) => {
                    self.set_pc(bus.get16(pc + 1));
                    self._cycles(4)
                }
                (0xe, 0x9) => {
                    self.set_pc(self.get_hl());
                    self._cycles(1)
                }
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
    use crate::bus::{Bus, ROM_N_END};
    use crate::cpu::Cpu;

    #[test]
    fn sp_signed() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();

        cpu.set_pc(0xB000);
        cpu.set_sp(0x000F);
        bus.set(0xB001, 1);
        cpu.misc((0xe, 8), &mut bus);
        assert_eq!(cpu.get_sp(), 0x0010);
        assert_eq!(cpu.get_flag_h(), true);
        assert_eq!(cpu.get_flag_c(), false);

        cpu.set_pc(0xB000);
        cpu.set_sp(0x00FF);
        bus.set(0xB001, 1);
        cpu.misc((0xe, 8), &mut bus);
        assert_eq!(cpu.get_sp(), 0x0100);
        assert_eq!(cpu.get_flag_h(), true);
        assert_eq!(cpu.get_flag_c(), true);

        cpu.set_pc(0xB000);
        cpu.set_sp(0x0000);
        bus.set(0xB001, 0xFF);
        cpu.misc((0xe, 8), &mut bus);
        assert_eq!(cpu.get_sp(), 0xFFFF);
        assert_eq!(cpu.get_flag_h(), false);
        assert_eq!(cpu.get_flag_c(), false);

        cpu.set_pc(0xB000);
        cpu.set_sp(0x0001);
        bus.set(0xB001, 0xFF);
        cpu.misc((0xe, 8), &mut bus);
        assert_eq!(cpu.get_sp(), 0x0000);
        assert_eq!(cpu.get_flag_h(), true);
        assert_eq!(cpu.get_flag_c(), true);
    }
    #[test]
    fn jump() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        let mut rng = rand::thread_rng();


        for _ in [..=30] {
            let x1 = rng.gen_range(0..255);
            let y1 = rng.gen_range(ROM_N_END..0xC000);
            cpu.set_pc(y1);
            bus.set(y1 + 1, x1);

            cpu.jump((0x1, 0x8), &mut bus);
            assert_eq!(cpu.get_pc(), y1.wrapping_add_signed((x1 as i8) as i16) + 2);
        }

        cpu.set_pc(0xB209);
        bus.set(0xB209 + 1, 0xFB);
        cpu.set_flag_z(false);

        cpu.jump((0x2, 0x0), &mut bus);
        assert_eq!(cpu.get_pc(), 0xB206);
    }
    #[test]
    fn load() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        let mut rng = rand::thread_rng();

        let x1 = rng.gen_range(0..255);
        let x2 = rng.gen_range(0..255);
        let x3 = rng.gen_range(0..255);
        let x4 = rng.gen_range(0..255);
        let x5 = rng.gen_range(ROM_N_END..=0xC000);

        cpu.c.set(x1);
        cpu.d.set(x2);

        cpu.load((0x5, 0x1), &mut bus);
        assert_eq!(cpu.c.get(), x1);
        assert_eq!(cpu.d.get(), x1);

        cpu.set_hl(x5);
        cpu.a.set(x3);

        cpu.load((0x2, 0x2), &mut bus);
        assert_eq!(bus.get(x5), x3);

        cpu.b.set(x4);
        cpu.load((0x7, 0x0), &mut bus);
        assert_eq!(bus.get(x5 + 1), x4);

        cpu.set_bc(x5 + 1);
        cpu.load((0x0, 0xa), &mut bus);
        assert_eq!(cpu.a.get(), x4);

        cpu.set_pc(x5);
        bus.set(cpu.get_pc() + 1, x1);
        cpu.load((1, 0xe), &mut bus);
        assert_eq!(cpu.e.get(), x1);
    }
    #[test]
    fn alu16() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();

        cpu.set_hl(0x4C00);
        cpu.alu16((0x2, 0x9), &mut bus);
        assert_eq!(cpu.get_flag_h(), true);
        assert_eq!(cpu.get_hl(), 0x9800);
    }
    #[test]
    fn alu() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        let mut rng = rand::thread_rng();

        let s2 = rng.gen_range(8..=15);
        let x1 = rng.gen_range(0..=255);
        let x2 = rng.gen_range(0..=255);
        let x3 = rng.gen_range(0..=255);
        let x4 = rng.gen_range(0..=127);
        let x5 = rng.gen_range(128..=255);
        let y1 = rng.gen_range(ROM_N_END..=0xC000);

        cpu.a.set(x1);
        cpu.d.set(x2);
        cpu.set_hl(y1);
        bus.set(y1, x3);

        cpu.alu((0x8, 0x2), &mut bus);
        assert_eq!(cpu.a.get(), x1.wrapping_add(x2));

        cpu.a.set(x1);
        cpu.alu((0x8, 0x6), &mut bus);
        assert_eq!(cpu.a.get(), x1.wrapping_add(x3));

        cpu.a.set(x1);
        cpu.alu((0x9, 0x2), &mut bus);
        assert_eq!(cpu.a.get(), x1.wrapping_sub(x2));

        cpu.a.set(x1);
        cpu.alu((0x9, 0x6), &mut bus);
        assert_eq!(cpu.a.get(), x1.wrapping_sub(x3));

        cpu.a.set(x1);
        cpu.set_flag_c(false);
        cpu.alu((0x8, 0xa), &mut bus);
        assert_eq!(cpu.a.get(), x1.wrapping_add(x2));

        cpu.a.set(x1);
        cpu.set_flag_c(true);
        cpu.alu((0x8, 0xe), &mut bus);
        assert_eq!(cpu.a.get(), x1.wrapping_add(x3 + 1));

        cpu.a.set(x1);
        cpu.set_flag_c(false);
        cpu.alu((0x9, 0xa), &mut bus);
        assert_eq!(cpu.a.get(), x1.wrapping_sub(x2));

        cpu.a.set(x1);
        cpu.set_flag_c(true);
        cpu.alu((0x9, 0xe), &mut bus);
        assert_eq!(cpu.a.get(), x1.wrapping_sub(x3 + 1));

        cpu.a.set(x1);
        cpu.alu((0xa, 0x2), &mut bus);
        assert_eq!(cpu.a.get(), x1 & x2);

        cpu.a.set(x1);
        cpu.alu((0xa, 0x6), &mut bus);
        assert_eq!(cpu.a.get(), x1 & x3);

        cpu.a.set(x1);
        cpu.alu((0xb, 0x2), &mut bus);
        assert_eq!(cpu.a.get(), x1 | x2);

        cpu.a.set(x1);
        cpu.alu((0xb, 0x6), &mut bus);
        assert_eq!(cpu.a.get(), x1 | x3);

        cpu.a.set(x1);
        cpu.alu((0xa, 0xa), &mut bus);
        assert_eq!(cpu.a.get(), x1 ^ x2);

        cpu.a.set(x1);
        cpu.alu((0xa, 0xe), &mut bus);
        assert_eq!(cpu.a.get(), x1 ^ x3);

        cpu.a.set(x4);
        cpu.b.set(x5);
        cpu.alu((0x9, 0), &mut bus);
        assert_eq!(cpu.get_flag_c(), true);
        assert_eq!(cpu.get_flag_n(), true);
        assert_eq!(cpu.get_flag_z(), x4 == x5);

        cpu.a.set(x4);
        cpu.b.set(x4);
        cpu.alu((0xb, 0x8), &mut bus);
        assert_eq!(cpu.get_flag_n(), true);
        assert_eq!(cpu.get_flag_z(), true);

        cpu.a.set(s2);
        cpu.b.set(s2);
        cpu.alu((0x8, 0), &mut bus);
        assert_eq!(cpu.get_flag_n(), false);
        assert_eq!(cpu.get_flag_h(), true);
    }

    #[test]
    fn inc() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();

        let x1 = 0x1;
        let x2 = 0xff;
        let x3 = 0x0f;

        cpu.c.set(x1);
        cpu.alu((0, 0xc), &mut bus);
        assert_eq!(cpu.get_flag_z(), false);
        assert_eq!(cpu.get_flag_n(), false);

        cpu.c.set(x2);
        cpu.alu((0, 0xc), &mut bus);
        assert_eq!(cpu.get_flag_z(), true);
        assert_eq!(cpu.get_flag_n(), false);

        cpu.c.set(x3);
        cpu.alu((0, 0xc), &mut bus);
        assert_eq!(cpu.get_flag_h(), true);
        assert_eq!(cpu.get_flag_n(), false);
    }
    #[test]
    fn dec() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();

        let x1 = 0x1;
        let x2 = 0xff;
        let x3 = 0x00;

        cpu.c.set(x2);
        cpu.alu((0, 0xd), &mut bus);
        assert_eq!(cpu.get_flag_z(), false);
        assert_eq!(cpu.get_flag_n(), true);

        cpu.c.set(x1);
        cpu.alu((0, 0xd), &mut bus);
        assert_eq!(cpu.get_flag_z(), true);
        assert_eq!(cpu.get_flag_n(), true);

        cpu.c.set(x3);
        cpu.alu((0, 0xd), &mut bus);
        assert_eq!(cpu.get_flag_h(), true);
        assert_eq!(cpu.get_flag_n(), true);
    }
    #[test]
    fn stack() {
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        let mut rng = rand::thread_rng();

        let x1 = rng.gen_range(0..=255);
        let x2 = rng.gen_range(0..=255);
        let y1 = rng.gen_range(0..=0xFFFF);
        let y2 = rng.gen_range(0..=0xFFFF);

        cpu.set_bc(y1);
        cpu.push((0xc, 5), &mut bus);
        cpu.pop((0xd, 1), &mut bus);
        assert_eq!(cpu.get_de(), y1);

        cpu.set_bc(y1);
        cpu.push((0xc, 5), &mut bus);
        cpu.set_bc(x1);
        cpu.push((0xc, 5), &mut bus);
        cpu.set_bc(y2);
        cpu.push((0xc, 5), &mut bus);
        cpu.set_bc(x2);
        cpu.push((0xc, 5), &mut bus);
        cpu.pop((0xd, 1), &mut bus);
        assert_eq!(cpu.get_de(), x2);
        cpu.pop((0xd, 1), &mut bus);
        assert_eq!(cpu.get_de(), y2);
        cpu.pop((0xd, 1), &mut bus);
        assert_eq!(cpu.get_de(), x1);
        cpu.pop((0xd, 1), &mut bus);
        assert_eq!(cpu.get_de(), y1);
    }
    fn sub_cycle(inst: u8, cycles: usize){
        let mut cpu = Cpu::new();
        let mut bus = Bus::new();
        cpu.set_pc(0xB000);
        cpu.set_hl(0xB000);
        cpu.set_bc(0xB000);
        cpu.set_de(0xB000);
        cpu.set_sp(0xB000);
        cpu.set_flag_z(false);
        cpu.set_flag_c(false);
        bus.set(cpu.get_pc(), inst);
        bus.set(cpu.get_pc() + 1, 0xBB);
        bus.set(cpu.get_pc() + 2, 0xBB);
        cpu.counter = 0;
        cpu.step(&mut bus, false);
        assert_eq!(cpu.counter, cycles);
    }
    #[test]
    fn cycles() {
        sub_cycle(0x00, 1);
        sub_cycle(0x01, 3);
        sub_cycle(0x02, 2);
        sub_cycle(0x03, 2);
        sub_cycle(0x04, 1);
        sub_cycle(0x05, 1);
        sub_cycle(0x06, 2);
        sub_cycle(0x07, 1);
        sub_cycle(0x08, 5);
        sub_cycle(0x09, 2);
        sub_cycle(0x0a, 2);
        sub_cycle(0x0b, 2);
        sub_cycle(0x0c, 1);
        sub_cycle(0x0d, 1);
        sub_cycle(0x0e, 2);
        sub_cycle(0x0f, 1);

        //sub_cycle(0x10, 1);
        sub_cycle(0x11, 3);
        sub_cycle(0x12, 2);
        sub_cycle(0x13, 2);
        sub_cycle(0x14, 1);
        sub_cycle(0x15, 1);
        sub_cycle(0x16, 2);
        sub_cycle(0x17, 1);
        sub_cycle(0x18, 3);
        sub_cycle(0x19, 2);
        sub_cycle(0x1a, 2);
        sub_cycle(0x1b, 2);
        sub_cycle(0x1c, 1);
        sub_cycle(0x1d, 1);
        sub_cycle(0x1e, 2);
        sub_cycle(0x1f, 1);

        sub_cycle(0x20, 3);
        sub_cycle(0x21, 3);
        sub_cycle(0x22, 2);
        sub_cycle(0x23, 2);
        sub_cycle(0x24, 1);
        sub_cycle(0x25, 1);
        sub_cycle(0x26, 2);
        sub_cycle(0x27, 1);
        sub_cycle(0x28, 2);
        sub_cycle(0x29, 2);
        sub_cycle(0x2a, 2);
        sub_cycle(0x2b, 2);
        sub_cycle(0x2c, 1);
        sub_cycle(0x2d, 1);
        sub_cycle(0x2e, 2);
        sub_cycle(0x2f, 1);

        sub_cycle(0x30, 3);
        sub_cycle(0x31, 3);
        sub_cycle(0x32, 2);
        sub_cycle(0x33, 2);
        sub_cycle(0x34, 3);
        sub_cycle(0x35, 3);
        sub_cycle(0x36, 3);
        sub_cycle(0x37, 1);
        sub_cycle(0x38, 2);
        sub_cycle(0x39, 2);
        sub_cycle(0x3a, 2);
        sub_cycle(0x3b, 2);
        sub_cycle(0x3c, 1);
        sub_cycle(0x3d, 1);
        sub_cycle(0x3e, 2);
        sub_cycle(0x3f, 1);

        sub_cycle(0x40, 1);
        sub_cycle(0x41, 1);
        sub_cycle(0x42, 1);
        sub_cycle(0x43, 1);
        sub_cycle(0x44, 1);
        sub_cycle(0x45, 1);
        sub_cycle(0x46, 2);
        sub_cycle(0x47, 1);
        sub_cycle(0x48, 1);
        sub_cycle(0x49, 1);
        sub_cycle(0x4a, 1);
        sub_cycle(0x4b, 1);
        sub_cycle(0x4c, 1);
        sub_cycle(0x4d, 1);
        sub_cycle(0x4e, 2);
        sub_cycle(0x4f, 1);

        sub_cycle(0x50, 1);
        sub_cycle(0x51, 1);
        sub_cycle(0x52, 1);
        sub_cycle(0x53, 1);
        sub_cycle(0x54, 1);
        sub_cycle(0x55, 1);
        sub_cycle(0x56, 2);
        sub_cycle(0x57, 1);
        sub_cycle(0x58, 1);
        sub_cycle(0x59, 1);
        sub_cycle(0x5a, 1);
        sub_cycle(0x5b, 1);
        sub_cycle(0x5c, 1);
        sub_cycle(0x5d, 1);
        sub_cycle(0x5e, 2);
        sub_cycle(0x5f, 1);

        sub_cycle(0x60, 1);
        sub_cycle(0x61, 1);
        sub_cycle(0x62, 1);
        sub_cycle(0x63, 1);
        sub_cycle(0x64, 1);
        sub_cycle(0x65, 1);
        sub_cycle(0x66, 2);
        sub_cycle(0x67, 1);
        sub_cycle(0x68, 1);
        sub_cycle(0x69, 1);
        sub_cycle(0x6a, 1);
        sub_cycle(0x6b, 1);
        sub_cycle(0x6c, 1);
        sub_cycle(0x6d, 1);
        sub_cycle(0x6e, 2);
        sub_cycle(0x6f, 1);

        sub_cycle(0x70, 2);
        sub_cycle(0x71, 2);
        sub_cycle(0x72, 2);
        sub_cycle(0x73, 2);
        sub_cycle(0x74, 2);
        sub_cycle(0x75, 2);
        //sub_cycle(0x76, 2);
        sub_cycle(0x77, 2);
        sub_cycle(0x78, 1);
        sub_cycle(0x79, 1);
        sub_cycle(0x7a, 1);
        sub_cycle(0x7b, 1);
        sub_cycle(0x7c, 1);
        sub_cycle(0x7d, 1);
        sub_cycle(0x7e, 2);
        sub_cycle(0x7f, 1);

        sub_cycle(0x80, 1);
        sub_cycle(0x81, 1);
        sub_cycle(0x82, 1);
        sub_cycle(0x83, 1);
        sub_cycle(0x84, 1);
        sub_cycle(0x85, 1);
        sub_cycle(0x86, 2);
        sub_cycle(0x87, 1);
        sub_cycle(0x88, 1);
        sub_cycle(0x89, 1);
        sub_cycle(0x8a, 1);
        sub_cycle(0x8b, 1);
        sub_cycle(0x8c, 1);
        sub_cycle(0x8d, 1);
        sub_cycle(0x8e, 2);
        sub_cycle(0x8f, 1);

        sub_cycle(0x90, 1);
        sub_cycle(0x91, 1);
        sub_cycle(0x92, 1);
        sub_cycle(0x93, 1);
        sub_cycle(0x94, 1);
        sub_cycle(0x95, 1);
        sub_cycle(0x96, 2);
        sub_cycle(0x97, 1);
        sub_cycle(0x98, 1);
        sub_cycle(0x99, 1);
        sub_cycle(0x9a, 1);
        sub_cycle(0x9b, 1);
        sub_cycle(0x9c, 1);
        sub_cycle(0x9d, 1);
        sub_cycle(0x9e, 2);
        sub_cycle(0x9f, 1);

        sub_cycle(0xa0, 1);
        sub_cycle(0xa1, 1);
        sub_cycle(0xa2, 1);
        sub_cycle(0xa3, 1);
        sub_cycle(0xa4, 1);
        sub_cycle(0xa5, 1);
        sub_cycle(0xa6, 2);
        sub_cycle(0xa7, 1);
        sub_cycle(0xa8, 1);
        sub_cycle(0xa9, 1);
        sub_cycle(0xaa, 1);
        sub_cycle(0xab, 1);
        sub_cycle(0xac, 1);
        sub_cycle(0xad, 1);
        sub_cycle(0xae, 2);
        sub_cycle(0xaf, 1);

        sub_cycle(0xb0, 1);
        sub_cycle(0xb1, 1);
        sub_cycle(0xb2, 1);
        sub_cycle(0xb3, 1);
        sub_cycle(0xb4, 1);
        sub_cycle(0xb5, 1);
        sub_cycle(0xb6, 2);
        sub_cycle(0xb7, 1);
        sub_cycle(0xb8, 1);
        sub_cycle(0xb9, 1);
        sub_cycle(0xba, 1);
        sub_cycle(0xbb, 1);
        sub_cycle(0xbc, 1);
        sub_cycle(0xbd, 1);
        sub_cycle(0xbe, 2);
        sub_cycle(0xbf, 1);

        sub_cycle(0xc0, 5);
        sub_cycle(0xc1, 3);
        sub_cycle(0xc2, 4);
        sub_cycle(0xc3, 4);
        sub_cycle(0xc4, 6);
        sub_cycle(0xc5, 4);
        sub_cycle(0xc6, 2);
        sub_cycle(0xc7, 4);
        sub_cycle(0xc8, 2);
        sub_cycle(0xc9, 4);
        sub_cycle(0xca, 3);
        sub_cycle(0xcc, 3);
        sub_cycle(0xcd, 6);
        sub_cycle(0xce, 2);
        sub_cycle(0xcf, 4);

        sub_cycle(0xd0, 5);
        sub_cycle(0xd1, 3);
        sub_cycle(0xd2, 4);
        sub_cycle(0xd4, 6);
        sub_cycle(0xd5, 4);
        sub_cycle(0xd6, 2);
        sub_cycle(0xd7, 4);
        sub_cycle(0xd8, 2);
        sub_cycle(0xd9, 4);
        sub_cycle(0xda, 3);
        sub_cycle(0xdc, 3);
        sub_cycle(0xde, 2);
        sub_cycle(0xdf, 4);

        sub_cycle(0xe0, 3);
        sub_cycle(0xe1, 3);
        sub_cycle(0xe2, 2);
        sub_cycle(0xe5, 4);
        sub_cycle(0xe6, 2);
        sub_cycle(0xe7, 4);
        sub_cycle(0xe8, 4);
        sub_cycle(0xe9, 1);
        sub_cycle(0xea, 4);
        sub_cycle(0xee, 2);
        sub_cycle(0xef, 4);

        sub_cycle(0xf0, 3);
        sub_cycle(0xf1, 3);
        sub_cycle(0xf2, 2);
        sub_cycle(0xf3, 1);
        sub_cycle(0xf5, 4);
        sub_cycle(0xf6, 2);
        sub_cycle(0xf7, 4);
        sub_cycle(0xf8, 3);
        sub_cycle(0xf9, 2);
        sub_cycle(0xfa, 4);
        sub_cycle(0xfb, 1);
        sub_cycle(0xfe, 2);
        sub_cycle(0xff, 4);
    }
}