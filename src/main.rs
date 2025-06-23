use std::fmt::Debug;
use std::io;
use std::ops::Deref;
use clap::Parser;
use crate::emulator::Emulator;
use peak_alloc::PeakAlloc;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

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
mod rom;
mod util;

const ROM: &[u8] = include_bytes!("../test-roms/Pokemon Red.gb");

fn main() {
    let game = rom::File::new("./test-roms/Pokemon Red.gb".to_string());
    let output = output::dummy::Dummy::new();
    //let output = output::lcd::LCD::new(4);
    let input = input::Dummy::new();
    let mut emu = Emulator::new(
        game,
        input,
        output,
    );

    emu.run(60*200, &mut io::stdout());
    let peak_mem = PEAK_ALLOC.peak_usage_as_kb();
    println!("The max amount that was used {}", peak_mem);
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::emulator::Emulator;
    use crate::input;
    use crate::output::dummy::Dummy;
    use crate::rom::File;

    #[test]
    fn blargg1() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("01-special.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg2() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("02-interrupts.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg3() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("03-op sp,hl.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg4() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("04-op r,imm.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg5() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("05-op rp.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg6() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("06-ld r,r.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg7() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("07-jr,jp,call,ret,rst.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg8() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("08-misc instrs.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg9() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("09-op r,r.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg10() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("10-bit ops.gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg11() {
        let mut emu = Emulator::new(File::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("11-op a,(hl).gb").to_str().unwrap().parse().unwrap()), input::Dummy::new(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(600, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
}