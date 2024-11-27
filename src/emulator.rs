use std::fs::File;
use std::io::{BufReader, Read};
use std::{io, thread, time};
use bitfield::Bit;
use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::fetcher::Fetcher;
use crate::output::{Output, LCD, Dummy};
use crate::ppu::{Ppu, PpuState};

pub struct Emulator<O: Output> {
    cpu: Cpu,
    bus: Bus,
    fetcher: Fetcher,
    ppu: Ppu,
    output: O,
}

impl<O: Output> Emulator<O> {
    pub fn new(rom_path: &str, output: O) -> Self {
        let rom = File::open(rom_path).expect("Could not open rom");

        let mut bus = Bus::new();
        let cpu = Cpu::new();
        let fetcher = Fetcher::new(&bus);
        let ppu = Ppu { state: PpuState::OAMFetch, oambuffer: Vec::new(), ticks: 0 };

        let mut reader = BufReader::new(rom);
        let mut buffer = Vec::new();
        let result = reader.read_to_end(&mut buffer);
        match result {
            Ok(_) => {}
            Err(_) => { panic!("oops") }
        }
        bus.load_rom(buffer);

        bus.set_int_enable_lcd(true);
        bus.set_int_enable_joypad(true);
        bus.set_int_enable_serial(true);
        bus.set_int_enable_vblank(true);
        bus.set_int_enable_timer(true);

        bus.set_int_request_lcd(false);
        bus.set_int_request_joypad(false);
        bus.set_int_request_serial(false);
        bus.set_int_request_vblank(false);
        bus.set_int_request_timer(false);

        Emulator { cpu, bus, fetcher, ppu, output }
    }

    pub fn run(&mut self, max_cycles: usize, stdout: &mut dyn io::Write) {
        let ten_millis = time::Duration::from_millis(1);
        let mut count: usize = 0;
        let mut timer: u64 = 0;
        loop {
            let cycles = match self.cpu.get_ime() {
                true => {
                    if self.bus.get_int_enable_vblank() && self.bus.get_int_request_vblank() {
                        self.bus.set_int_request_vblank(false);
                        self.cpu.interrupt(&mut self.bus, 0x40)
                    } else if self.bus.get_int_enable_lcd() && self.bus.get_int_request_lcd() {
                        self.bus.set_int_request_lcd(false);
                        self.cpu.interrupt(&mut self.bus, 0x48)
                    } else if self.bus.get_int_enable_timer() && self.bus.get_int_request_timer() {
                        self.bus.set_int_request_timer(false);
                        self.cpu.interrupt(&mut self.bus, 0x50)
                    } else if self.bus.get_int_enable_serial() && self.bus.get_int_request_serial() {
                        self.bus.set_int_request_serial(false);
                        self.cpu.interrupt(&mut self.bus, 0x58)
                    } else if self.bus.get_int_enable_joypad() && self.bus.get_int_request_joypad() {
                        self.bus.set_int_request_joypad(false);
                        self.cpu.interrupt(&mut self.bus, 0x60)
                    } else {
                        self.cpu.step(&mut self.bus)
                    }
                }
                false => self.cpu.step(&mut self.bus)
            };
            timer += cycles as u64;

            for _ in 1..=cycles * 4 {
                self.ppu.tick(&mut self.bus, &mut self.fetcher, &mut self.output);
            }
            for _ in 1..=cycles {
                if timer % 64 == 0{
                    self.bus.set(0xFF04, self.bus.get(0xFF04).overflowing_add(1).0);
                }

                if self.bus.get(0xFF07).bit(2) {
                    let step_size = match self.bus.get(0xFF07) & 0x3 {
                        0 => 256,
                        1 => 4,
                        2 => 16,
                        3 => 64,
                        _ => panic!("Should be impossible!")
                    };
                    if timer % step_size == 0 {
                        let (val, c) = self.bus.get(0xFF05).overflowing_add(1);
                        self.bus.set(0xFF05, val);
                        if c {
                            self.bus.set(0xFF05, self.bus.get(0xFF06));
                            self.bus.set_int_request_timer(true);
                        }
                    }
                }
            }

            if self.bus.get(0xFF02) == 0x81 {
                write!(stdout, "{}", self.bus.get(0xFF01) as char).expect("Couldn't write");
                self.bus.set(0xFF02, 0);
            }
            if count % 1024 == 0 {
                //thread::sleep(ten_millis);
            }
            count += 1;
            if count > max_cycles && max_cycles != 0 {
                break;
            }
        }
    }
}