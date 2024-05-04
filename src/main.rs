use crate::emulator::Emulator;

mod cpu;
mod bus;
mod emulator;
mod register;
mod memory;
mod ppu;
mod fetcher;
mod output;

fn main() {
    let mut emu = Emulator::new("C:\\Users\\Sofyan\\RustroverProjects\\rusty-gb\\test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\05-op rp.gb");

    emu.run();
}
