# rusty-gb

A Game Boy (DMG) emulator in Rust, targeting both a desktop host and a
Raspberry Pi Pico 2 (RP2350, Cortex-M33) from one source tree.

## Cartridge images are not included

No commercial ROMs are distributed with this repository. `test-roms/*.gb` is
gitignored — supply your own dumps of cartridges you own.

The host binary reads its ROM at runtime, but **the Pico build needs one at
compile time**: `src/main.rs` does

```rust
static ROM_IMAGE: &[u8] = include_bytes!("../test-roms/Pokemon Red.gb");
```

so that the image lands in `.rodata` and can be executed in place from XIP
flash, costing no SRAM. Drop a file at that path or edit the `include_bytes!`
to point at your own. Without it the Pico build fails at compile time with a
"couldn't read" error naming the missing path.

Note that a built Pico image therefore *embeds* the cartridge — don't
redistribute the resulting `.elf` or `.uf2`.

What *is* included is `test-roms/gb-test-roms-master/`, blargg's freely
redistributable test suite. That is all the test suite needs.

## Building

The host is the default target, so the test suite is always one command away:

```sh
cargo test              # 34 tests, incl. blargg cpu_instrs 1-11
cargo run --release     # needs test-roms/Pokemon Red.gb
```

The Pico 2 is one `--target` away:

```sh
cargo build --target thumbv8m.main-none-eabihf --profile embedded
cargo run   --target thumbv8m.main-none-eabihf --profile embedded  # flashes via picotool
```

`cargo run` for the Pico uses `picotool`, which needs the board in BOOTSEL
mode. Either hold BOOTSEL while plugging it in, or — once firmware with the
1200 baud reset is on the board — let the host ask for it (see below).

The split is on `target_os = "none"`: the Pico target reports `none`, hosts
report macos/linux/windows, so neither build needs a remembered `--features`
to be correct.

### The Pico runs at 300 MHz

`src/clocks.rs` replaces `hal::clocks::init_clocks_and_plls`, which brings the
RP2350 up at its specified 150 MHz, with the same sequence at double that. The
emulator is CPU-bound on this board — about 30 fps at stock — so the frame rate
follows the clock, though short of a clean 2×: the flash divisor is scaled with
the clock (below), so XIP cache misses cost the same wall-clock time they did
before while everything else halves.

Three things move, in this order: core voltage to 1.15 V, then the QSPI flash
divisor, then the PLL. `SYS_MHZ` in that file is the only knob — set it back to
`150` and the PLL config and flash divisor follow, putting the board at stock.

**300 MHz is out of spec.** Raspberry Pi specify 150 MHz at 1.10 V. Silicon
varies, and a part that will not hold this shows it as a hang or a corrupted
frame, not as an error message. If that happens, try 1.20 V (`VSEL` `0b01101`)
before suspecting anything else, and drop `SYS_MHZ` if it persists.

## Talking to the Pico

Diagnostics go out over **USB CDC**, not defmt/RTT, so reading them needs no
debug probe — just the cable that already powers the board:

```sh
ls /dev/tty.usbmodem*            # macOS; /dev/ttyACM* on Linux
screen /dev/tty.usbmodem101      # baud is ignored for CDC; exit with Ctrl-A K
```

You get a live FPS line every 15 frames plus a final average. The emulator
waits up to 10 seconds for a terminal to open the port before starting, so
nothing is lost off the top, and still boots unattended if nothing attaches.

The first line out of the port is the system clock the PLL actually locked to,
so an overclock that silently fell back is visible immediately rather than only
as disappointing FPS.

The Game Boy's own serial port goes to the same physical port but a separate
sink, so a parser reading cartridge output never sees an FPS line spliced into
it.

## Flashing without the BOOTSEL button

The firmware watches the CDC port for the 1200 baud touch — open the port at
1200 baud with DTR low and it reboots into BOOTSEL — so the host can put the
board into flashing mode itself. `picotool` then loads over the same cable, and
the terminal comes back on its own.

On Windows, `tools/flash.ps1` does the whole loop:

```powershell
.\tools\flash.ps1              # build, reset, flash, attach
.\tools\flash.ps1 -MonitorOnly # attach to a board that is already running
.\tools\flash.ps1 -NoBuild     # flash what is already in target/
.\tools\flash.ps1 -Reset       # just drop into BOOTSEL, then exit
```

It finds the board by USB VID/PID (`2E8A:000A`), so no COM port has to be
remembered; pass `-Port COM5` if two boards are plugged in. It needs `picotool`
on `PATH` — except under `-MonitorOnly`, which needs nothing but the port.
Ctrl-C detaches the monitor and leaves the board running.

The same trick from a Unix shell, if you are not on Windows:

```sh
stty -f /dev/tty.usbmodem101 1200      # macOS; stty -F /dev/ttyACM0 1200 on Linux
cargo run --target thumbv8m.main-none-eabihf --profile embedded
```

A board whose firmware predates this still needs the button held at plug-in —
including the first flash after checking this out.
