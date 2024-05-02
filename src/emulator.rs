use std::fs::File;
use std::io::{BufReader, Read};
use std::thread;
use std::time;
use crate::cpu::Cpu;

pub struct Emulator {
    cpu: Cpu
}

impl Emulator {
    pub fn new(rom_path: &str) -> Emulator {
        let rom = File::open(rom_path).expect("Could not open rom");
        let mut cpu = Cpu::new();

        let mut reader = BufReader::new(rom);
        let mut buffer = Vec::new();
        let result = reader.read_to_end(&mut buffer);
        match result {
            Ok(_) => {}
            Err(_) => {panic!("oops")}
        }
        cpu.bus.load_rom(buffer);

        Emulator{cpu}
    }

    pub fn run(&mut self) {
        let ten_millis = time::Duration::from_millis(20);
        loop {
            self.cpu.step();
            if self.cpu.bus.get(0xFF02) == 1 {
                print!("{}", self.cpu.bus.get(0xFF01) as char);
                self.cpu.bus.set(0xFF02, 0);
            }
            //thread::sleep(ten_millis);
        }
    }
}