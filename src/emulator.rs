use std::fs::File;
use std::io::{BufReader, Read};
use std::{io, time};
use bitfield::Bit;
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
        output.init();

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
        let ten_millis = time::Duration::from_millis(20);
        let mut count : usize = 0;
        let mut timer : usize = 0;
        loop {
            let cycles = match self.cpu.get_ime() {
                true => {
                    if self.bus.get_int_enable_vblank() && self.bus.get_int_request_vblank() {
                        self.cpu.interrupt(&mut self.bus, 0x40)
                    } else if self.bus.get_int_enable_lcd() && self.bus.get_int_request_lcd() {
                        self.cpu.interrupt(&mut self.bus, 0x48)
                    } else if self.bus.get_int_enable_timer() && self.bus.get_int_request_timer() {
                        self.cpu.interrupt(&mut self.bus, 0x50)
                    } else if self.bus.get_int_enable_serial() && self.bus.get_int_request_serial() {
                        self.cpu.interrupt(&mut self.bus, 0x58)
                    } else if self.bus.get_int_enable_joypad() && self.bus.get_int_request_joypad() {
                        self.cpu.interrupt(&mut self.bus, 0x60)
                    } else {
                        self.cpu.step(&mut self.bus)
                    }
                }
                false => self.cpu.step(&mut self.bus)
            };
            timer += cycles;

            for _ in 1..=cycles {
                self.ppu = self.ppu.execute(&mut self.bus, &mut self.fetcher, &self.output);
            }
            if timer % 64 == 0 {
                self.bus.set(0xFF04, self.bus.get(0xFF04).overflowing_add(1).0);
            }
            if self.bus.get(0xFF07).bit(2){
                let step_size = match self.bus.get(0xFF07) & 0x3 {
                    0 => 256,
                    1 => 4,
                    2 => 16,
                    3 => 64,
                    _ => panic!("Should be impossible!")
                };
                if timer % step_size == 0 {
                    let (val, c) = self.bus.get(0xFF04).overflowing_add(1);
                    self.bus.set(0xFF05, val);
                    if c{
                        self.bus.set(0xFF05, self.bus.get(0xFF06));
                        self.bus.set_int_request_timer(true);
                    }

                }
            }

            if self.bus.get(0xFF02) == 0x81 {
                //write!(stdout, "{}", self.bus.get(0xFF01) as char).expect("Couldn't write");
                self.bus.set(0xFF02, 0);
            }
            //thread::sleep(ten_millis);
            count += 1;
            if count > max_cycles && max_cycles != 0 {
                break;
            }
        }
    }
}