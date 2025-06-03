use defmt::println;
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
        println!("A: {:02X} F: {:02X} B: {:02X} C: {:02X} D: {:02X} E: {:02X} H: {:02X} L: {:02X} SP: {:04X} PC: 00:{:04X} ({:02X} {:02X} {:02X} {:02X}) LY: {:02X}",
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
                 bus.get(self.get_pc() + 3),
                 bus.get(0xFF44)
        )
    }
    pub fn step(&mut self, mut bus: &mut Bus, log: bool) -> usize {
        //self.log(bus);

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
