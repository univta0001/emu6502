//#![windows_subsystem = "windows"]

use emu6502::bus::Bus;
use emu6502::cpu::{CpuStats, CPU};
use std::error::Error;
use std::io::{self, BufWriter, Write};

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
use std::fs::File;

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
use pprof::protos::Message;

#[rustfmt::skip]
fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let bus = Bus::default();
    let mut cpu = CPU::new(bus);
    let mut _cpu_stats = CpuStats::new();

    let mut instr_count: usize = 0;

    //let function_test: Vec<u8> = std::fs::read("6502_functional_test.bin").unwrap();
    let function_test: Vec<u8> = include_bytes!("../../6502_functional_test.bin").to_vec();
    //let _function_test: Vec<u8> = std::fs::read("65C02_extended_opcodes_test.bin").unwrap();
    cpu.load(&function_test, 0x0);
    cpu.reset();
    cpu.program_counter = 0x0400;
    cpu.bus.disable_video = true;
    cpu.bus.disable_audio = true;
    cpu.bus.disable_disk = true;
    cpu.self_test = true;
    //cpu.m65c02 = true;

    let stdout = io::stdout();
    let mut _handle = BufWriter::new(stdout.lock());
    let mut _output = String::new();

    let now = std::time::Instant::now();

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    let guard = pprof::ProfilerGuard::new(100).unwrap();

    cpu.run_with_callback(|_cpu| {
        instr_count += 1;
        _cpu_stats.update(_cpu);
    });

    let elapsed = now.elapsed().as_millis();
    let estimated_mhz = (cpu.bus.get_cycles() as f32 / elapsed as f32) / 1000.0;

    let mut output = io::stderr();
    writeln!(output, "Elapsed time:      {elapsed:>12} ms")?;
    writeln!(output, "Estimated MHz:    {estimated_mhz:>12.3} MHz")?;
    writeln!(output, "Total Cycles:         {:>12}", cpu.bus.get_cycles())?;
    writeln!(output, "Total Instructions:   {instr_count:>12}")?;

    writeln!(output, "Total Branches:       {:>12}", _cpu_stats.branches)?;
    writeln!(output, "Total Branches Taken: {:>12}", _cpu_stats.branches_taken)?;
    writeln!(output, "Branches cross-page:  {:>12}", _cpu_stats.branches_cross_page)?;
    writeln!(output, "Absolute X cross-page:{:>12}", _cpu_stats.absolute_x_cross_page)?;
    writeln!(output, "Absolute Y cross-page:{:>12}", _cpu_stats.absolute_y_cross_page)?;
    writeln!(output, "Indirect Y cross-page:{:>12}", _cpu_stats.indirect_y_cross_page)?;

    // Save the pprof output
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    {
        if let Ok(report) = guard.report().build() {
            let flame_file = File::create("flamegraph.svg").unwrap();
            report.flamegraph(flame_file).unwrap();

            let mut file = File::create("profile.pb").unwrap();
            let profile = report.pprof().unwrap();

            let mut content = Vec::new();
            profile.write_to_vec(&mut content).unwrap();
            file.write_all(&content).unwrap();
        }
    }

    Ok(())
}
