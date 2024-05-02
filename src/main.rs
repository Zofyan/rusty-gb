use crate::emulator::Emulator;

mod cpu;
mod bus;
mod emulator;
mod register;

fn main() {
    let mut emu = Emulator::new("C:\\Users\\Sofyan\\RustroverProjects\\rusty-gb\\test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\07-jr,jp,call,ret,rst.gb");

    emu.run();
}
