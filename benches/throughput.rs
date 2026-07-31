//! How fast the emulator runs, in the units an emulator is actually judged in.
//!
//! ```sh
//! cargo bench                              # run, and compare to the saved baseline
//! cargo bench -- --save main               # record this run as the baseline "main"
//! cargo bench -- --baseline main           # compare against a named baseline
//! cargo bench -- --json                    # emit machine-readable results
//! cargo bench -- cpu_instrs                # only workloads matching a filter
//! ```
//!
//! # What is measured
//!
//! The headline figure is **x real time**: emulated M-cycles per second of wall
//! clock, over the 1048576 a DMG manages. It is the only number that compares
//! across machines, across ROMs and against the Pico, and it is what decides
//! whether a change is worth having.
//!
//! Each workload runs a fixed number of emulated cycles, so every repetition
//! executes exactly the same instructions -- these are deterministic programs
//! with no input. That makes the fastest repetition the right estimator: the
//! work is identical, so anything above the minimum is the host's noise and not
//! the emulator's.
//!
//! Two output backends run for each ROM, because they separate two costs that
//! the frame rate alone confuses:
//!
//! * `dummy` throws scanlines away -- the interpreter and the PPU's own work.
//! * `frame` writes all 160x144 pixels through [`Framebuffer`] -- the same, plus
//!   the per-scanline handover a real backend pays.
//!
//! # Why the frame rate here disagrees with the one the binary prints
//!
//! [`Emulator::run`] reports frames as calls to [`Emulator::batch`], and a batch
//! is [`STEPS_PER_BATCH`] *instructions*, not a frame's worth of cycles. At the
//! DMG's average of a little over three M-cycles per instruction, one batch is
//! roughly three frames of emulated time -- so the FPS the binary prints is
//! about a third of the frames actually being emulated. This prints both, and
//! `cycles/batch` against [`CYCLES_PER_FRAME`] is the ratio between them.
//!
//! No harness and no dependencies: `cargo bench`'s own harness is unstable, and
//! what this needs -- fixed work, best-of-N, a saved baseline -- is a hundred
//! lines.

use rusty_gb::emulator::{Emulator, CYCLES_PER_FRAME, STEPS_PER_BATCH};
use rusty_gb::input;
use rusty_gb::output::dummy::Dummy;
use rusty_gb::output::Output;
use rusty_gb::rom::Rom;
use rusty_gb::testrom::{Framebuffer, CYCLES_PER_SECOND};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Emulated seconds per repetition. Long enough that a run is tens of
/// milliseconds -- well clear of timer granularity and of the cost of loading
/// the cartridge -- and short enough that the whole suite is a few seconds.
const WORKLOAD_SECS: u64 = 5;

/// Repetitions per workload. The minimum is the estimator, so this only has to
/// be enough reps for one of them to land on a quiet moment.
const REPETITIONS: u32 = 7;

/// A change smaller than this is not reported as a change. Wall-clock timing on
/// a general-purpose host does not resolve better than a couple of percent, and
/// a benchmark that cries regression at 0.5% gets ignored.
const NOISE_FLOOR_PCT: f64 = 3.0;

/// ROMs to run, and why each is here.
///
/// All vendored, so this needs no cartridge dump of anyone's: the point is a
/// stable mix of work, not a particular game. Between them they cover an
/// interpreter-bound load, a rendering-bound one, and a mapper.
const WORKLOADS: &[(&str, &str)] = &[
    // Wall-to-wall ALU and branches with the screen almost static: the closest
    // thing to a pure interpreter benchmark in the suite.
    ("cpu_instrs", "gb-test-roms-master/cpu_instrs/cpu_instrs.gb"),
    // Drives the PPU hard -- it exists to provoke sprite and scanline
    // behaviour -- so the rendering path dominates instead.
    ("oam_bug", "gb-test-roms-master/oam_bug/oam_bug.gb"),
    // Bank switching on every other access, which is the one path where the
    // mapper is not just noise.
    ("mbc1", "mooneye-test-suite/emulator-only/mbc1/bits_bank1.gb"),
];

/// Discards the Game Boy's serial output. The ROMs here write a few hundred
/// bytes over tens of millions of instructions, but formatting them into a
/// `String` that nothing reads would still be measured work.
struct Sink;

impl std::fmt::Write for Sink {
    fn write_str(&mut self, _: &str) -> std::fmt::Result {
        Ok(())
    }
}

/// One measurement.
struct Measurement {
    workload: String,
    /// Emulated M-cycles executed, identical across repetitions by construction.
    cycles: u64,
    /// Instructions retired, likewise.
    instructions: u64,
    /// The fastest repetition.
    best: Duration,
    /// The slowest, as a read on how noisy the host was.
    worst: Duration,
}

impl Measurement {
    fn cycles_per_sec(&self) -> f64 {
        self.cycles as f64 / self.best.as_secs_f64()
    }

    /// Emulated speed as a multiple of a real DMG.
    fn realtime(&self) -> f64 {
        self.cycles_per_sec() / CYCLES_PER_SECOND as f64
    }

    /// True emulated frames per second -- cycles, not batches.
    fn frames_per_sec(&self) -> f64 {
        self.cycles_per_sec() / CYCLES_PER_FRAME as f64
    }

    fn instructions_per_sec(&self) -> f64 {
        self.instructions as f64 / self.best.as_secs_f64()
    }

    /// Spread between the fastest and slowest repetition, as a percentage of
    /// the fastest. A large value means the number below it is not to be
    /// trusted to the digit.
    fn spread_pct(&self) -> f64 {
        (self.worst.as_secs_f64() / self.best.as_secs_f64() - 1.0) * 100.0
    }
}

/// Runs one ROM for a fixed number of emulated cycles against a given output
/// backend, and times it.
fn measure<O: Output, F: Fn() -> O>(workload: &str, rom: &Path, make_output: F) -> Measurement {
    let budget = WORKLOAD_SECS * CYCLES_PER_SECOND;
    let mut best = Duration::MAX;
    let mut worst = Duration::ZERO;
    let mut cycles = 0;
    let mut instructions = 0;

    // One untimed repetition first: the first run of a workload pays for the
    // page faults on the ROM image and for a cold instruction cache, neither of
    // which is the emulator's cost.
    for repetition in 0..=REPETITIONS {
        let mut emu = Emulator::new(
            Rom::file(rom.to_str().expect("rom path is not utf-8")),
            input::Dummy::new(),
            make_output(),
        );

        let started = Instant::now();
        let mut retired = 0u64;
        while emu.cycles() < budget {
            emu.step(&mut Sink);
            retired += 1;
        }
        let elapsed = started.elapsed();

        if repetition == 0 {
            cycles = emu.cycles();
            instructions = retired;
            continue;
        }

        // The work is identical every time; if it is not, the emulator is not
        // deterministic and every number here is meaningless.
        assert_eq!(emu.cycles(), cycles, "{workload}: cycle count varied between runs");
        assert_eq!(retired, instructions, "{workload}: instruction count varied between runs");

        best = best.min(elapsed);
        worst = worst.max(elapsed);
    }

    Measurement { workload: workload.to_string(), cycles, instructions, best, worst }
}

// ============================== baselines ==============================

/// Where a baseline lives. Under `target/` because it describes one machine and
/// one build, and is meaningless checked in next to source that outlives both.
fn baseline_path(name: &str) -> PathBuf {
    Path::new("target").join("bench-baselines").join(format!("{name}.txt"))
}

/// One line per workload: name, cycles per second, instructions per second.
/// Deliberately not JSON -- reading it back would want a parser, and this is
/// three fields. `--json` is there for anything that wants structure.
fn save_baseline(name: &str, results: &[Measurement]) -> std::io::Result<PathBuf> {
    let path = baseline_path(name);
    std::fs::create_dir_all(path.parent().expect("baseline path has a parent"))?;

    let mut out = String::from("# rusty-gb throughput baseline\n# workload\tcycles/s\tinstructions/s\n");
    for result in results {
        let _ = writeln!(
            out,
            "{}\t{:.0}\t{:.0}",
            result.workload,
            result.cycles_per_sec(),
            result.instructions_per_sec()
        );
    }
    std::fs::write(&path, out)?;
    Ok(path)
}

/// Cycles-per-second by workload, or `None` if the baseline does not exist.
fn load_baseline(name: &str) -> Option<Vec<(String, f64)>> {
    let text = std::fs::read_to_string(baseline_path(name)).ok()?;
    Some(
        text.lines()
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let workload = fields.next()?.to_string();
                let rate = fields.next()?.parse().ok()?;
                Some((workload, rate))
            })
            .collect(),
    )
}

// ================================ output ================================

fn report(results: &[Measurement], baseline: Option<(&str, Vec<(String, f64)>)>) {
    println!(
        "\n{:<22} {:>11} {:>9} {:>9} {:>11} {:>7}",
        "workload", "Mcycles/s", "x real", "fps", "Minstr/s", "spread"
    );
    println!("{}", "-".repeat(74));

    for result in results {
        println!(
            "{:<22} {:>11.1} {:>9.1} {:>9.0} {:>11.1} {:>6.1}%",
            result.workload,
            result.cycles_per_sec() / 1e6,
            result.realtime(),
            result.frames_per_sec(),
            result.instructions_per_sec() / 1e6,
            result.spread_pct(),
        );
    }

    // Cycles per instruction is a property of the ROM, not of the host, so it
    // is the one figure here that should not move when the machine is busy --
    // and the one that says how far a batch is from a frame.
    let cycles: u64 = results.iter().map(|r| r.cycles).sum();
    let instructions: u64 = results.iter().map(|r| r.instructions).sum();
    let per_instruction = cycles as f64 / instructions as f64;
    println!(
        "\n{:.2} M-cycles per instruction, so one batch of {STEPS_PER_BATCH} instructions is\n\
         {:.0} cycles: {:.2} frames of emulated time, not one. The frame rate the binary\n\
         prints is refreshes per second, i.e. about {:.2}x lower than true emulated fps.",
        per_instruction,
        STEPS_PER_BATCH as f64 * per_instruction,
        STEPS_PER_BATCH as f64 * per_instruction / CYCLES_PER_FRAME as f64,
        STEPS_PER_BATCH as f64 * per_instruction / CYCLES_PER_FRAME as f64,
    );

    let Some((name, baseline)) = baseline else {
        return;
    };

    println!("\nagainst baseline {name:?}:");
    let mut compared = 0;
    for result in results {
        let Some((_, was)) = baseline.iter().find(|(workload, _)| *workload == result.workload)
        else {
            println!("  {:<22} (not in baseline)", result.workload);
            continue;
        };
        compared += 1;
        let delta = (result.cycles_per_sec() / was - 1.0) * 100.0;
        let verdict = if delta.abs() < NOISE_FLOOR_PCT {
            "unchanged"
        } else if delta > 0.0 {
            "FASTER"
        } else {
            "SLOWER"
        };
        println!("  {:<22} {:>+7.1}%  {verdict}", result.workload, delta);
    }
    if compared == 0 {
        println!("  (no workloads in common -- was the baseline saved with a different filter?)");
    }
}

fn report_json(results: &[Measurement]) {
    println!("{{");
    println!("  \"cycles_per_second_of_a_dmg\": {CYCLES_PER_SECOND},");
    println!("  \"workloads\": [");
    for (index, result) in results.iter().enumerate() {
        let comma = if index + 1 == results.len() { "" } else { "," };
        println!(
            "    {{ \"name\": {:?}, \"cycles\": {}, \"instructions\": {}, \
             \"best_seconds\": {:.6}, \"cycles_per_second\": {:.0}, \"realtime\": {:.3} }}{comma}",
            result.workload,
            result.cycles,
            result.instructions,
            result.best.as_secs_f64(),
            result.cycles_per_sec(),
            result.realtime(),
        );
    }
    println!("  ]");
    println!("}}");
}

// ================================= main =================================

/// The value following `flag`, if it was given.
fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let at = args.iter().position(|arg| arg == flag)?;
    args.get(at + 1).map(|s| s.as_str())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|arg| arg == "--json");
    let save = flag_value(&args, "--save");
    let compare_to = flag_value(&args, "--baseline").unwrap_or("main");

    // `cargo bench` appends `--bench` itself, and `--save`/`--baseline` consume
    // the word after them; everything else that is not a flag is a filter.
    let consumed: Vec<&str> = ["--save", "--baseline"]
        .iter()
        .filter_map(|flag| flag_value(&args, flag))
        .collect();
    let filters: Vec<&str> = args
        .iter()
        .map(|s| s.as_str())
        .filter(|arg| !arg.starts_with('-') && !consumed.contains(arg))
        .collect();
    let wanted = |name: &str| filters.is_empty() || filters.iter().any(|f| name.contains(f));

    let mut results = Vec::new();
    for (name, rom) in WORKLOADS {
        let path = Path::new("test-roms").join(rom);
        assert!(
            path.exists(),
            "{} is missing -- the test ROM suites are vendored, so this means the \
             working tree is incomplete rather than that a download is needed",
            path.display()
        );

        for backend in ["dummy", "frame"] {
            let workload = format!("{name}/{backend}");
            if !wanted(&workload) {
                continue;
            }
            if !json {
                eprint!("  measuring {workload}...\r");
            }
            results.push(match backend {
                "dummy" => measure(&workload, &path, Dummy::new),
                _ => measure(&workload, &path, Framebuffer::new),
            });
        }
    }

    if results.is_empty() {
        eprintln!("no workloads matched {filters:?}");
        std::process::exit(1);
    }

    if json {
        report_json(&results);
        return;
    }

    let baseline = load_baseline(compare_to).map(|loaded| (compare_to, loaded));
    if baseline.is_none() {
        eprintln!("(no baseline {compare_to:?} yet -- `cargo bench -- --save {compare_to}` to record one)");
    }
    report(&results, baseline);

    if let Some(name) = save {
        match save_baseline(name, &results) {
            Ok(path) => println!("\nsaved baseline {name:?} to {}", path.display()),
            Err(error) => eprintln!("\ncould not save baseline: {error}"),
        }
    }
}
