use std::io;
use std::path::Path;
use crate::emulator::Emulator;
use crate::output::LCD;

mod cpu;
mod bus;
mod emulator;
mod register;
mod memory;
mod ppu;
mod fetcher;
mod output;
mod window_fetcher;

fn main() {
    let mut output = LCD::new(5f64);
    let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual")
        //.join("02-interrupts.gb").to_str().unwrap());
        //.join("09-op r,r.gb").to_str().unwrap());
        .join("07-jr,jp,call,ret,rst.gb").to_str().unwrap(), output);
        //.join("11-op a,(hl).gb").to_str().unwrap());

    emu.run(0, &mut io::stdout());
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::emulator::Emulator;
    use crate::output::Dummy;

    #[test]
    fn blargg1() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("01-special.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg2() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("02-interrupts.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg3() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("03-op sp,hl.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg4() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("04-op r,imm.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg5() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("05-op rp.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg6() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("06-ld r,r.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg7() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("07-jr,jp,call,ret,rst.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg8() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("08-misc instrs.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg9() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("09-op r,r.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg10() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("10-bit ops.gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg11() {
        let mut emu = Emulator::new(Path::new("test-roms").join("gb-test-roms-master").join("cpu_instrs").join("individual").join("11-op a,(hl).gb").to_str().unwrap(), Dummy::new());
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
}