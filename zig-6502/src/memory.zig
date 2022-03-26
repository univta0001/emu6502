const std = @import("std");
const Type = @import("types.zig").Type;
const testing = std.testing;

const allocator = std.heap.page_allocator;

pub const Memory = struct {
    const SIZE = 2 << 16;

    data: [SIZE]Type.Byte,

    pub fn init() Memory {
        var self = Memory{
            .data = undefined,
        };
        self.clear();
        return self;
    }

    pub fn clear(self: *Memory) void {
        for (self.data) |*d| {
            d.* = 0;
        }
    }
};

test "create memory" {
    var memory = Memory.init();
    try testing.expect(memory.data[0] == 0);
    try testing.expect(memory.data[Memory.SIZE - 1] == 0);
    memory.data[0x1234] = 0x11;
    try testing.expect(memory.data[0x1234] == 0x11);
}
