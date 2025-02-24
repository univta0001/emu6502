use criterion::{Criterion, criterion_group, criterion_main};
use emu6502::bus::Bus;
use emu6502::cpu::CPU;
use std::time::Duration;

fn bench_cpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("emu6502");
    let significance_level = 0.05;
    let duration = 15;
    let sample_size = 200;
    group
        .significance_level(significance_level)
        .confidence_level(1.0 - significance_level)
        .measurement_time(Duration::new(duration, 0))
        .sample_size(sample_size);
    group.bench_function("cpu_wait_fca8", |b| {
        b.iter(|| {
            // FCA8 Wait routine has 2.5 A^2 + 13.5 A + 13 cycles
            // Each cycle takes 14 / 14.318181 microseconds
            // For A=0, it is like A=256 but 10 cycles short. It should take 167299 CPU cycles.
            let bus = Bus::default();
            let mut cpu = CPU::new(bus);
            cpu.bench_test = true;
            cpu.bus.disable_audio = true;
            cpu.bus.disable_video = true;
            cpu.bus.disable_disk = true;
            cpu.load_and_run(&[
                0x20, 0x04, 0x00, 0x00, 0x38, 0x48, 0xe9, 0x01, 0xd0, 0xfc, 0x68, 0xe9, 0x01, 0xd0,
                0xf6, 0x60,
            ]);
        })
    });
}

criterion_group!(benches, bench_cpu);

criterion_main!(benches);
