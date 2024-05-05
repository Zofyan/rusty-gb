use std::io;
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
    let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\09-op r,r.gb");

    emu.run(0, &mut io::stdout());
}

#[cfg(test)]
mod tests {
    use crate::emulator::Emulator;
    #[test]
    fn blargg1() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\01-special.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg2() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\02-interrupts.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg3() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\03-op sp,hl.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg4() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\04-op r,imm.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg5() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\05-op rp.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg6() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\06-ld r,r.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg7() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\07-jr,jp,call,ret,rst.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg8() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\08-misc instrs.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg9() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\09-op r,r.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg10() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\10-bit ops.gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
    #[test]
    fn blargg11() {
        let mut emu = Emulator::new("test-roms\\gb-test-roms-master\\cpu_instrs\\individual\\11-op a,(hl).gb");
        let mut stdout = Vec::new();

        emu.run(10_000_000, &mut stdout);

        let output = String::from_utf8_lossy(&*stdout);
        assert_eq!(output.contains("Passed"), true);
        assert_eq!(output.contains("Failed"), false);
    }
}