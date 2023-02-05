const std = @import("std");
const Type = @import("types.zig").Type;
const CPU = @import("cpu.zig").CPU;

pub fn main() !void {
    const address: Type.Word = 0x0400;
    var cpu = CPU.init();
    cpu.reset(address);

    var file = try std.fs.cwd().openFile("data/6502_functional_test.bin", .{});
    defer file.close();

    const file_size = try file.getEndPos();
    std.debug.print("File size {d}\n", .{file_size});
    const bytes_read = try file.read(cpu.memory.data[0x000A .. 0x000A + file_size]);
    _ = bytes_read;
    var lastPC: Type.Word = 0;

    const start_time = std.time.milliTimestamp();

    while (true) {
        // run for at least 1 tick
        const used = cpu.run(1);
        _ = used;
        //std.debug.print("Ran for {} ticks, PC is now {x}\n", .{ used, cpu.PC });
        if (lastPC == cpu.PC) {
            // std.debug.print("STUCK AT 0x{x}\n", .{cpu.PC});
            break;
        }
        lastPC = cpu.PC;
    }

    const end_time = std.time.milliTimestamp();

    std.debug.print("Elapsed time = {d} ms \n", .{(end_time-start_time)});
}
