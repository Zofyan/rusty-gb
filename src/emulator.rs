use std::fs::File;
use std::io::{BufReader, Read};
use std::time;
use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::fetcher::Fetcher;
use crate::output::{Output, LCD};
use crate::ppu::{Ppu, OAMFetch};

pub struct Emulator {
    cpu: Cpu,
    bus: Bus,
    fetcher: Fetcher,
    ppu: Box<dyn Ppu>,
    output: Box<dyn Output>,
}

impl Emulator {
    pub fn new(rom_path: &str) -> Emulator {
        let rom = File::open(rom_path).expect("Could not open rom");

        let mut bus = Bus::new();
        let mut cpu = Cpu::new();
        let mut fetcher = Fetcher::new();
        let mut ppu = Box::new(OAMFetch {ticks: 0});
        let output = Box::new(LCD {});

        let mut reader = BufReader::new(rom);
        let mut buffer = Vec::new();
        let result = reader.read_to_end(&mut buffer);
        match result {
            Ok(_) => {}
            Err(_) => { panic!("oops") }
        }
        bus.load_rom(buffer);

        Emulator { cpu, bus, fetcher, ppu, output }
    }

    pub fn run(&mut self) {
        let ten_millis = time::Duration::from_millis(20);
        loop {
            let cycles = match self.cpu.get_ime() {
                true => {
                    if self.bus.get_int_enable_vblank() && self.bus.get_int_request_vblank() {
                        self.cpu.interrupt(&mut self.bus, 0x40)
                    } else if self.bus.get_int_enable_lcd() && self.bus.get_int_enable_lcd() {
                        self.cpu.interrupt(&mut self.bus, 0x48)
                    } else if self.bus.get_int_enable_timer() && self.bus.get_int_enable_timer() {
                        self.cpu.interrupt(&mut self.bus, 0x50)
                    } else if self.bus.get_int_enable_serial() && self.bus.get_int_enable_serial() {
                        self.cpu.interrupt(&mut self.bus, 0x58)
                    } else if self.bus.get_int_enable_joypad() && self.bus.get_int_enable_joypad() {
                        self.cpu.interrupt(&mut self.bus, 0x60)
                    } else {
                        self.cpu.step(&mut self.bus)
                    }
                }
                false => self.cpu.step(&mut self.bus)
            };

            for _ in 1..=cycles {
                self.ppu = self.ppu.execute(&mut self.bus, &mut self.fetcher, &self.output);
            }

            if self.bus.get(0xFF02) > 0 {
                //print!("{}", self.bus.get(0xFF01) as char);
                self.bus.set(0xFF02, 0);
            }
            //thread::sleep(ten_millis);
        }
    }
}