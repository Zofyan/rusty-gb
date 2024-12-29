use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::fetcher::Fetcher;
use crate::input::Input;
use crate::output::{Dummy, Output, LCD};
use crate::ppu::{Ppu, PpuState};
use crate::window_fetcher::WindowFetcher;
use bitfield::Bit;
use macroquad::prelude::next_frame;
use std::fs::File;
use std::io::{BufReader, Read};
use std::{io, thread, time};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Emulator<O: Output, I: Input> {
    cpu: Cpu,
    bus: Bus,
    ppu: Ppu,
    output: O,
    input: I,
    fps: Vec<f64>,
}

impl<O: Output, I: Input> Emulator<O, I> {
    pub fn new(rom_path: &str, input: I, output: O) -> Self {
        let rom = File::open(rom_path).expect("Could not open rom");

        let mut bus = Bus::new();
        let cpu = Cpu::new();
        let ppu = Ppu::new();

        let mut reader = BufReader::new(rom);
        let mut buffer = Vec::new();
        let result = reader.read_to_end(&mut buffer);
        match result {
            Ok(_) => {}
            Err(_) => {
                panic!("oops")
            }
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

        Emulator {
            cpu,
            bus,
            ppu,
            output,
            input,
            fps: vec![],
        }
    }

    pub fn run(&mut self, max_cycles: usize, stdout: &mut dyn io::Write) {
        let ten_millis = time::Duration::from_millis(1);
        let mut count: usize = 0;
        let mut timer: u64 = 0;
        loop {
            let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
            for _ in 0..17476 {
                self.input.check_input(&mut self.bus);
                let cycles = match self.cpu.get_ime() {
                    true => {
                        if self.bus.get_int_enable_vblank() && self.bus.get_int_request_vblank() {
                            self.bus.set_int_request_vblank(false);
                            self.cpu.interrupt(&mut self.bus, 0x40)
                        } else if self.bus.get_int_enable_lcd() && self.bus.get_int_request_lcd() {
                            self.bus.set_int_request_lcd(false);
                            self.cpu.interrupt(&mut self.bus, 0x48)
                        } else if self.bus.get_int_enable_timer() && self.bus.get_int_request_timer()
                        {
                            self.bus.set_int_request_timer(false);
                            self.cpu.interrupt(&mut self.bus, 0x50)
                        } else if self.bus.get_int_enable_serial() && self.bus.get_int_request_serial()
                        {
                            self.bus.set_int_request_serial(false);
                            self.cpu.interrupt(&mut self.bus, 0x58)
                        } else if self.bus.get_int_enable_joypad() && self.bus.get_int_request_joypad()
                        {
                            self.bus.set_int_request_joypad(false);
                            self.cpu.interrupt(&mut self.bus, 0x60)
                        } else {
                            self.cpu.step(&mut self.bus, true)
                        }
                    }
                    false => self.cpu.step(&mut self.bus, true),
                };
                timer += cycles as u64;

                for _ in 1..=cycles * 4 {
                    self.ppu.tick(&mut self.bus, &mut self.output);
                }
                for _ in 1..=cycles {
                    if timer % 64 == 0 {
                        self.bus.inc(0xFF04);
                    }

                    if self.bus.get(0xFF07).bit(2) {
                        let step_size = match self.bus.get(0xFF07) & 0x3 {
                            0 => 256,
                            1 => 4,
                            2 => 16,
                            3 => 64,
                            _ => panic!("Should be impossible!"),
                        };
                        if timer % step_size == 0 {
                            let val = self.bus.get(0xFF05).wrapping_add(1);
                            self.bus.set(0xFF05, val);
                            if val == 0 {
                                self.bus.set(0xFF05, self.bus.get(0xFF06));
                                self.bus.set_int_request_timer(true);
                            }
                        }
                    }
                }

                if self.bus.get(0xFF02) == 0x81 {
                    write!(stdout, "{}", self.bus.get(0xFF01) as char).expect("Couldn't write");
                    stdout.flush().expect("Couldn't flush");
                    self.bus.set(0xFF02, 0);
                }
            }
            count += 1;
            if count > max_cycles && max_cycles != 0 {
                println!("Avg FPS: {:}", self.fps.iter().sum::<f64>() / self.fps.len() as f64);
                break;
            }
            // next_frame().await;
            let diff = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - millis;
            self.fps.push(1000.0 / diff as f64);
            println!("FPS: {}", 1000.0 / diff as f64);
        }
    }
}
