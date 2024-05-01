use crate::emulator::Emulator;

mod cpu;
mod bus;
mod emulator;
mod register;

fn main() {
    let mut emu = Emulator::new();

    emu.run();

}
