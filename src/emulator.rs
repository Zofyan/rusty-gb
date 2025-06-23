use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::input::Input;
use crate::output::Output;
use crate::ppu::{Ppu};
use bitfield::Bit;
use std::io::{Write};
use std::time::{SystemTime, UNIX_EPOCH};
use cloneable_file::CloneableFile;
use crate::rom::ROM;

pub struct Emulator<I: Input, O: Output> {
    cpu: Cpu,
    bus: Bus,
    ppu: Ppu,
    output: O,
    input: I,
    fps: Vec<f64>,
}

impl<I: Input, O: Output> Emulator<I, O> {
    pub fn new<R: ROM + 'static>(game: R, input: I, output: O) -> Self {

        let mut bus = Bus::new();
        let cpu = Cpu::new();
        let ppu = Ppu::new();

        bus.load_rom(game);

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

    pub fn run(&mut self, max_cycles: usize, stdout: &mut dyn Write) {
        let mut count: usize = 0;
        let mut timer: u64 = 0;
        loop {
            let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
            for i in 0..17476 {
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

                self.ppu.tick(&mut self.bus, &mut self.output, cycles);

                for _ in 1..=cycles {
                    if timer % 64 == 0 {
                        self.bus.registers.div = self.bus.registers.div.wrapping_add(1);
                    }

                    if self.bus.registers.tca.bit(2) {
                        let step_size = match self.bus.registers.tca & 0x3 {
                            0 => 256,
                            1 => 4,
                            2 => 16,
                            3 => 64,
                            _ => panic!("Should be impossible!"),
                        };
                        if timer % step_size == 0 {
                            let val = self.bus.registers.tima.wrapping_add(1);
                            self.bus.registers.tima = val;
                            if val == 0 {
                                self.bus.registers.tima = self.bus.registers.tma;
                                self.bus.set_int_request_timer(true);
                            }
                        }
                    }
                }

                if self.bus.registers.sc == 0x81 {
                    write!(stdout, "{}", self.bus.registers.sb as char).expect("Couldn't write");
                    stdout.flush().expect("Couldn't flush");
                    self.bus.registers.sc = 0;
                }
            }
            count += 1;
            if count > max_cycles && max_cycles != 0 {
                println!("Avg FPS: {:}", self.fps.iter().sum::<f64>() / self.fps.len() as f64);
                break;
            }
            if !self.output.refresh() {
                break;
            }

            let diff = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() - millis;

            let time = 1_000_000.0 / diff as f64;
            self.fps.push(time);
            if time < 1_000_000.0 / 60.0 {
                //sleep(Duration::from_micros((1_000_000.0 / 60.0 - time) as u64))
            }
            self.output.set_diagnostics(format!("FPS: {}", time).parse().unwrap());
        }
    }
}
