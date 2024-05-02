use crate::emulator::Emulator;

mod cpu;
mod bus;
mod emulator;
mod register;

fn main() {
    let mut emu = Emulator::new("/home/saarrass/gb-test-roms/cpu_instrs/individual/06-ld r,r.gb");

    emu.run();
}
