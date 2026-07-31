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

What *is* included are the two freely redistributable test suites, vendored so
that `cargo test` needs no network and no setup:

* `test-roms/gb-test-roms-master/` — blargg's suite
* `test-roms/mooneye-test-suite/` — Gekkio's mooneye suite (MIT, `LICENSE`
  included), release `mts-20240926-1737-443f6e1`

## Building

The host is the default target, so the test suite is always one command away:

```sh
cargo test --release    # 69 tests; see Testing below for the other 125
cargo run --release     # needs test-roms/Pokemon Red.gb
```

`--release` is worth typing: the ROM suites run a few hundred million emulated
instructions, which takes about two seconds optimised and rather longer not.

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

## Testing

The emulator core is a library (`src/lib.rs`); `src/main.rs` is the two entry
points and the Pico's board glue. That split is what lets `tests/` and
`benches/` link the same code the binary runs.

```sh
cargo test --release                       # 69 tests, all passing
cargo test --release -- --ignored          # the 125 known-failing ROMs
cargo test --release --test mooneye        # one suite
```

There are four layers, in rough order of how much they will tell you:

| what | where | what it catches |
| --- | --- | --- |
| **ROM suites** | `tests/blargg.rs`, `tests/mooneye.rs` | the emulator disagreeing with real hardware |
| **snapshots** | `tests/snapshot.rs` | the emulator disagreeing with *itself* — any drift in cycles, pixels or registers |
| **opcode timing** | `src/cpu.rs`, `mod timing` | an instruction costing the wrong number of M-cycles |
| **unit tests** | throughout `src/` | the pieces, including a full opcode-dispatch equivalence check |

### The ROM suites, and why most of them are ignored

157 ROMs, one `#[test]` each so a regression names the ROM that broke. 32 pass
(15 blargg, 17 mooneye). The other 125 are `#[ignore]`d, each carrying the reason
it cannot pass rather than a bare "known failure":

```
#[ignore = "no APU (fail)"]
#[ignore = "PPU is stepped per instruction, not per dot (timeout)"]
#[ignore = "MBC5 not implemented (crashed)"]
```

Those reasons come down to two things. There is no APU at all, and the emulator
advances the PPU, the timer and the bus one *instruction* at a time rather than
one cycle at a time — so anything measuring when within an instruction an effect
lands will fail. Notably this is **not** a question of instruction cycle counts:
`mod timing` checks all 512 opcodes against the DMG's published costs, both arms
of every conditional branch included, and they are all correct.

To re-derive the list after a fix — which is the only way it stays honest:

```sh
cargo run --release --example test-report            # a table of every ROM
cargo run --release --example test-report -- mooneye # one suite
cargo run --release --example test-report -- --rust  # the declarations, to paste
```

The trade in using `#[ignore]` is that a ROM which *starts* passing stays quietly
ignored. `cargo test -- --ignored` is what surfaces those, and CI runs it on
every push as a non-blocking step for exactly that reason.

Two suites, two protocols, both in `src/testrom.rs`. blargg's `cpu_instrs`
generation reports over the serial port; everything later reports through
cartridge RAM at `$A000` and says nothing over serial at all. mooneye reports in
the register file at a `LD B,B` breakpoint. `every_vendored_rom_is_declared`
fails if a ROM is added to `test-roms/` without a test.

## Benchmarking

```sh
cargo bench                          # compare against the saved baseline
cargo bench -- --save main           # record this build as the baseline
cargo bench -- --json                # machine-readable
cargo bench -- cpu_instrs            # filter
```

Each workload runs a fixed number of *emulated* cycles, so every repetition does
identical work — the benchmark asserts as much — and the fastest of seven is the
estimator. Three ROMs against two output backends separate interpreter cost from
the cost of handing scanlines to a backend.

```
workload                 Mcycles/s    x real       fps    Minstr/s  spread
--------------------------------------------------------------------------
cpu_instrs/dummy              82.4      78.6      4696        39.2    2.6%
cpu_instrs/frame              79.7      76.0      4539        37.8    2.5%
oam_bug/dummy                 93.2      88.8      5306        38.8    8.1%
```

**`x real` is the number that matters**: emulated M-cycles per second over the
1048576 a DMG manages. It is the only figure that compares across hosts, across
ROMs, and against the Pico.

Baselines live in `target/bench-baselines/`, so they are per-machine and
per-build and are not checked in. A difference under 3% is reported as
unchanged; wall-clock timing on a general-purpose host does not resolve better
than that.

### The FPS the binary prints is not frames per second

`Emulator::run` reports one "frame" per call to `Emulator::batch`, and a batch is
17476 *instructions*, not a frame's worth of cycles. The benchmark measures the
actual ratio — about 2.25 M-cycles per instruction, so a batch is around 2.25
frames of emulated time. The live readout over USB CDC is therefore roughly a
third of the frames really being emulated. The number is consistent with itself
and fine for spotting a slowdown; it is just not fps.

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
