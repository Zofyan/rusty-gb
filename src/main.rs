use std::fmt::Debug;
use std::io;
use std::path::Path;
use crate::emulator::Emulator;
use crate::output::{Dummy, Terminal, LCD, LCDD};
use macroquad::prelude::*;
use crate::input::Controller;

mod cpu;
mod bus;
mod emulator;
mod register;
mod memory;
mod ppu;
mod fetcher;
mod output;
mod window_fetcher;
mod input;
mod mbc;

#[macroquad::main("Emulator")]
async fn main() {
    let output = LCDD::new(5f64);
    let input = input::Controller::new();
    let mut emu = Emulator::new(
        Path::new("test-roms").join("Pokemon Red.gb").to_str().unwrap(),
        input,
        output,
    );

    emu.run(3000, &mut io::stdout()).await;
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::emulator::Emulator;
    use crate::input;
    use crate::output::Dummy;

    #[test]
    fn blargg1() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("01-special.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg2() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("02-interrupts.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg3() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("03-op sp,hl.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg4() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("04-op r,imm.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg5() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("05-op rp.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg6() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("06-ld r,r.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg7() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("07-jr,jp,call,ret,rst.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg8() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("08-misc instrs.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg9() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("09-op r,r.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg10() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("10-bit ops.gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg11() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("11-op a,(hl).gb").to_str().unwrap(), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
}