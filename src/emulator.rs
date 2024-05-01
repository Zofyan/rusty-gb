use crate::cpu::Cpu;

pub struct Emulator {
    cpu: Cpu
}

impl Emulator {
    pub fn new() -> Emulator {
        let cpu = Cpu::new();
        Emulator{cpu}
    }

    pub fn run(&mut self) {
        loop {
            self.cpu.step();
        }
    }
}