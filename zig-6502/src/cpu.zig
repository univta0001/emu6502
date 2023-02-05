const std = @import("std");
const Type = @import("types.zig").Type;
const Status = @import("status.zig").Status;
const Memory = @import("memory.zig").Memory;
const testing = std.testing;

const allocator = std.heap.page_allocator;

pub const CPU = struct {
    const INITIAL_ADDRESS = 0xF000;
    const STACK_BASE = 0x0100;

    PC: Type.Word, // Program Counter
    PS: Status, // Processor Status

    regs: [4]Type.Byte,
    memory: Memory, // Memory bank with 64 KB -- wow

    ticks: u32, // Cycle counter

    // TODO: I would like these to be members of an enum, and have an array of
    // them.
    const A = 0;
    const X = 1;
    const Y = 2;
    const SP = 3;

    const DisplaceOp = enum {
        ShiftLeft,
        ShiftRight,
        RotateLeft,
        RotateRight,
    };

    const NumOp = enum {
        Add,
        Subtract,
    };

    const BitOp = enum {
        And,
        InclusiveOr,
        ExclusiveOr,
        Bit,
    };

    const IncDecOp = enum {
        Increment,
        Decrement,
    };

    const ClearSet = enum {
        Clear,
        Set,
    };

    const AddressingMode = enum {
        Immediate,
        ZeroPage,
        ZeroPageX,
        ZeroPageY,
        Absolute,
        AbsoluteX,
        AbsoluteY,
        Indirect,
        IndirectX,
        IndirectY,
    };

    const OP = enum(Type.Byte) {
        LDA_IMM = 0xA9,
        LDA_ZP = 0xA5,
        LDA_ZPX = 0xB5,
        LDA_ABS = 0xAD,
        LDA_ABSX = 0xBD,
        LDA_ABSY = 0xB9,
        LDA_INDX = 0xA1,
        LDA_INDY = 0xB1,

        LDX_IMM = 0xA2,
        LDX_ZP = 0xA6,
        LDX_ZPY = 0xB6,
        LDX_ABS = 0xAE,
        LDX_ABSY = 0xBE,

        LDY_IMM = 0xA0,
        LDY_ZP = 0xA4,
        LDY_ZPX = 0xB4,
        LDY_ABS = 0xAC,
        LDY_ABSX = 0xBC,

        STA_ZP = 0x85,
        STA_ZPX = 0x95,
        STA_ABS = 0x8D,
        STA_ABSX = 0x9D,
        STA_ABSY = 0x99,
        STA_INDX = 0x81,
        STA_INDY = 0x91,

        STX_ZP = 0x86,
        STX_ZPY = 0x96,
        STX_ABS = 0x8E,

        STY_ZP = 0x84,
        STY_ZPX = 0x94,
        STY_ABS = 0x8C,

        TAX = 0xAA,
        TAY = 0xA8,
        TXA = 0x8A,
        TYA = 0x98,
        TSX = 0xBA,
        TXS = 0x9A,

        PHA = 0x48,
        PHP = 0x08,
        PLA = 0x68,
        PLP = 0x28,

        AND_IMM = 0x29,
        AND_ZP = 0x25,
        AND_ZPX = 0x35,
        AND_ABS = 0x2D,
        AND_ABSX = 0x3D,
        AND_ABSY = 0x39,
        AND_INDX = 0x21,
        AND_INDY = 0x31,

        EOR_IMM = 0x49,
        EOR_ZP = 0x45,
        EOR_ZPX = 0x55,
        EOR_ABS = 0x4D,
        EOR_ABSX = 0x5D,
        EOR_ABSY = 0x59,
        EOR_INDX = 0x41,
        EOR_INDY = 0x51,

        ORA_IMM = 0x09,
        ORA_ZP = 0x05,
        ORA_ZPX = 0x15,
        ORA_ABS = 0x0D,
        ORA_ABSX = 0x1D,
        ORA_ABSY = 0x19,
        ORA_INDX = 0x01,
        ORA_INDY = 0x11,

        BIT_ZP = 0x24,
        BIT_ABS = 0x2C,

        ADC_IMM = 0x69,
        ADC_ZP = 0x65,
        ADC_ZPX = 0x75,
        ADC_ABS = 0x6D,
        ADC_ABSX = 0x7D,
        ADC_ABSY = 0x79,
        ADC_INDX = 0x61,
        ADC_INDY = 0x71,

        SBC_IMM = 0xE9,
        SBC_ZP = 0xE5,
        SBC_ZPX = 0xF5,
        SBC_ABS = 0xED,
        SBC_ABSX = 0xFD,
        SBC_ABSY = 0xF9,
        SBC_INDX = 0xE1,
        SBC_INDY = 0xF1,

        CMP_IMM = 0xC9,
        CMP_ZP = 0xC5,
        CMP_ZPX = 0xD5,
        CMP_ABS = 0xCD,
        CMP_ABSX = 0xDD,
        CMP_ABSY = 0xD9,
        CMP_INDX = 0xC1,
        CMP_INDY = 0xD1,

        CPX_IMM = 0xE0,
        CPX_ZP = 0xE4,
        CPX_ABS = 0xEC,

        CPY_IMM = 0xC0,
        CPY_ZP = 0xC4,
        CPY_ABS = 0xCC,

        INX = 0xE8,
        INY = 0xC8,
        INC_ZP = 0xE6,
        INC_ZPX = 0xF6,
        INC_ABS = 0xEE,
        INC_ABSX = 0xFE,

        DEX = 0xCA,
        DEY = 0x88,
        DEC_ZP = 0xC6,
        DEC_ZPX = 0xD6,
        DEC_ABS = 0xCE,
        DEC_ABSX = 0xDE,

        CLC = 0x18,
        CLD = 0xD8,
        CLI = 0x58,
        CLV = 0xB8,
        SEC = 0x38,
        SED = 0xF8,
        SEI = 0x78,

        ASL_ACC = 0x0A,
        ASL_ZP = 0x06,
        ASL_ZPX = 0x16,
        ASL_ABS = 0x0E,
        ASL_ABSX = 0x1E,

        LSR_ACC = 0x4A,
        LSR_ZP = 0x46,
        LSR_ZPX = 0x56,
        LSR_ABS = 0x4E,
        LSR_ABSX = 0x5E,

        ROL_ACC = 0x2A,
        ROL_ZP = 0x26,
        ROL_ZPX = 0x36,
        ROL_ABS = 0x2E,
        ROL_ABSX = 0x3E,

        ROR_ACC = 0x6A,
        ROR_ZP = 0x66,
        ROR_ZPX = 0x76,
        ROR_ABS = 0x6E,
        ROR_ABSX = 0x7E,

        BCC = 0x90,
        BCS = 0xB0,
        BEQ = 0xF0,
        BMI = 0x30,
        BNE = 0xD0,
        BPL = 0x10,
        BVC = 0x50,
        BVS = 0x70,

        JMP_ABS = 0x4C,
        JMP_IND = 0x6C,
        JSR_ABS = 0x20,
        RTS = 0x60,

        BRK = 0x00,
        RTI = 0x40,
        NOP = 0xEA,
    };

    pub fn init() CPU {
        var self = CPU{
            .PC = undefined,
            .PS = undefined,
            .regs = undefined,
            .memory = undefined,
            .ticks = undefined,
        };
        self.reset(INITIAL_ADDRESS);
        return self;
    }

    pub fn reset(self: *CPU, address: Type.Word) void {
        self.PC = address;
        self.PS.clear();
        self.regs[SP] = 0xFF;
        self.regs[A] = 0;
        self.regs[X] = 0;
        self.regs[Y] = 0;
        self.memory.clear();
        self.ticks = 0;
    }

    pub fn run(self: *CPU, limit: u32) u32 {
        const start = self.ticks;
        while ((self.ticks - start) < limit) {
            const op = @intToEnum(OP, self.readByte(self.PC));
            self.PC += 1;
            switch (op) {
                OP.LDA_IMM => self.fetch(A, .Immediate),
                OP.LDA_ZP => self.fetch(A, .ZeroPage),
                OP.LDA_ZPX => self.fetch(A, .ZeroPageX),
                OP.LDA_ABS => self.fetch(A, .Absolute),
                OP.LDA_ABSX => self.fetch(A, .AbsoluteX),
                OP.LDA_ABSY => self.fetch(A, .AbsoluteY),
                OP.LDA_INDX => self.fetch(A, .IndirectX),
                OP.LDA_INDY => self.fetch(A, .IndirectY),

                OP.LDX_IMM => self.fetch(X, .Immediate),
                OP.LDX_ZP => self.fetch(X, .ZeroPage),
                OP.LDX_ZPY => self.fetch(X, .ZeroPageY),
                OP.LDX_ABS => self.fetch(X, .Absolute),
                OP.LDX_ABSY => self.fetch(X, .AbsoluteY),

                OP.LDY_IMM => self.fetch(Y, .Immediate),
                OP.LDY_ZP => self.fetch(Y, .ZeroPage),
                OP.LDY_ZPX => self.fetch(Y, .ZeroPageX),
                OP.LDY_ABS => self.fetch(Y, .Absolute),
                OP.LDY_ABSX => self.fetch(Y, .AbsoluteX),

                OP.STA_ZP => self.store(A, .ZeroPage),
                OP.STA_ZPX => self.store(A, .ZeroPageX),
                OP.STA_ABS => self.store(A, .Absolute),
                OP.STA_ABSX => self.store(A, .AbsoluteX),
                OP.STA_ABSY => self.store(A, .AbsoluteY),
                OP.STA_INDX => self.store(A, .IndirectX),
                OP.STA_INDY => self.store(A, .IndirectY),

                OP.STX_ZP => self.store(X, .ZeroPage),
                OP.STX_ZPY => self.store(X, .ZeroPageY),
                OP.STX_ABS => self.store(X, .Absolute),

                OP.STY_ZP => self.store(Y, .ZeroPage),
                OP.STY_ZPX => self.store(Y, .ZeroPageX),
                OP.STY_ABS => self.store(Y, .Absolute),

                OP.TAX => self.transfer(A, X, true),
                OP.TAY => self.transfer(A, Y, true),
                OP.TXA => self.transfer(X, A, true),
                OP.TYA => self.transfer(Y, A, true),
                OP.TSX => self.transfer(SP, X, true),
                OP.TXS => self.transfer(X, SP, false),

                OP.PHA => self.pushRegisterToStack(A),
                OP.PHP => self.pushPSToStack(),
                OP.PLA => self.popRegisterFromStack(A),
                OP.PLP => self.popPSFromStack(true),

                OP.AND_IMM => self.bitOp(.And, A, .Immediate),
                OP.AND_ZP => self.bitOp(.And, A, .ZeroPage),
                OP.AND_ZPX => self.bitOp(.And, A, .ZeroPageX),
                OP.AND_ABS => self.bitOp(.And, A, .Absolute),
                OP.AND_ABSX => self.bitOp(.And, A, .AbsoluteX),
                OP.AND_ABSY => self.bitOp(.And, A, .AbsoluteY),
                OP.AND_INDX => self.bitOp(.And, A, .IndirectX),
                OP.AND_INDY => self.bitOp(.And, A, .IndirectY),

                OP.EOR_IMM => self.bitOp(.ExclusiveOr, A, .Immediate),
                OP.EOR_ZP => self.bitOp(.ExclusiveOr, A, .ZeroPage),
                OP.EOR_ZPX => self.bitOp(.ExclusiveOr, A, .ZeroPageX),
                OP.EOR_ABS => self.bitOp(.ExclusiveOr, A, .Absolute),
                OP.EOR_ABSX => self.bitOp(.ExclusiveOr, A, .AbsoluteX),
                OP.EOR_ABSY => self.bitOp(.ExclusiveOr, A, .AbsoluteY),
                OP.EOR_INDX => self.bitOp(.ExclusiveOr, A, .IndirectX),
                OP.EOR_INDY => self.bitOp(.ExclusiveOr, A, .IndirectY),

                OP.ORA_IMM => self.bitOp(.InclusiveOr, A, .Immediate),
                OP.ORA_ZP => self.bitOp(.InclusiveOr, A, .ZeroPage),
                OP.ORA_ZPX => self.bitOp(.InclusiveOr, A, .ZeroPageX),
                OP.ORA_ABS => self.bitOp(.InclusiveOr, A, .Absolute),
                OP.ORA_ABSX => self.bitOp(.InclusiveOr, A, .AbsoluteX),
                OP.ORA_ABSY => self.bitOp(.InclusiveOr, A, .AbsoluteY),
                OP.ORA_INDX => self.bitOp(.InclusiveOr, A, .IndirectX),
                OP.ORA_INDY => self.bitOp(.InclusiveOr, A, .IndirectY),

                OP.BIT_ZP => self.bitOp(.Bit, A, .ZeroPage),
                OP.BIT_ABS => self.bitOp(.Bit, A, .Absolute),

                OP.ADC_IMM => self.numOp(.Add, A, .Immediate),
                OP.ADC_ZP => self.numOp(.Add, A, .ZeroPage),
                OP.ADC_ZPX => self.numOp(.Add, A, .ZeroPageX),
                OP.ADC_ABS => self.numOp(.Add, A, .Absolute),
                OP.ADC_ABSX => self.numOp(.Add, A, .AbsoluteX),
                OP.ADC_ABSY => self.numOp(.Add, A, .AbsoluteY),
                OP.ADC_INDX => self.numOp(.Add, A, .IndirectX),
                OP.ADC_INDY => self.numOp(.Add, A, .IndirectY),

                OP.SBC_IMM => self.numOp(.Subtract, A, .Immediate),
                OP.SBC_ZP => self.numOp(.Subtract, A, .ZeroPage),
                OP.SBC_ZPX => self.numOp(.Subtract, A, .ZeroPageX),
                OP.SBC_ABS => self.numOp(.Subtract, A, .Absolute),
                OP.SBC_ABSX => self.numOp(.Subtract, A, .AbsoluteX),
                OP.SBC_ABSY => self.numOp(.Subtract, A, .AbsoluteY),
                OP.SBC_INDX => self.numOp(.Subtract, A, .IndirectX),
                OP.SBC_INDY => self.numOp(.Subtract, A, .IndirectY),

                OP.CMP_IMM => self.compareRegister(A, .Immediate),
                OP.CMP_ZP => self.compareRegister(A, .ZeroPage),
                OP.CMP_ZPX => self.compareRegister(A, .ZeroPageX),
                OP.CMP_ABS => self.compareRegister(A, .Absolute),
                OP.CMP_ABSX => self.compareRegister(A, .AbsoluteX),
                OP.CMP_ABSY => self.compareRegister(A, .AbsoluteY),
                OP.CMP_INDX => self.compareRegister(A, .IndirectX),
                OP.CMP_INDY => self.compareRegister(A, .IndirectY),

                OP.CPX_IMM => self.compareRegister(X, .Immediate),
                OP.CPX_ZP => self.compareRegister(X, .ZeroPage),
                OP.CPX_ABS => self.compareRegister(X, .Absolute),

                OP.CPY_IMM => self.compareRegister(Y, .Immediate),
                OP.CPY_ZP => self.compareRegister(Y, .ZeroPage),
                OP.CPY_ABS => self.compareRegister(Y, .Absolute),

                OP.INX => self.incDecReg(.Increment, X),
                OP.INY => self.incDecReg(.Increment, Y),
                OP.INC_ZP => self.incDecMem(.Increment, .ZeroPage),
                OP.INC_ZPX => self.incDecMem(.Increment, .ZeroPageX),
                OP.INC_ABS => self.incDecMem(.Increment, .Absolute),
                OP.INC_ABSX => self.incDecMem(.Increment, .AbsoluteX),

                OP.DEX => self.incDecReg(.Decrement, X),
                OP.DEY => self.incDecReg(.Decrement, Y),
                OP.DEC_ZP => self.incDecMem(.Decrement, .ZeroPage),
                OP.DEC_ZPX => self.incDecMem(.Decrement, .ZeroPageX),
                OP.DEC_ABS => self.incDecMem(.Decrement, .Absolute),
                OP.DEC_ABSX => self.incDecMem(.Decrement, .AbsoluteX),

                OP.CLC => self.clearSetBit(.Clear, .Carry),
                OP.CLD => self.clearSetBit(.Clear, .Decimal),
                OP.CLI => self.clearSetBit(.Clear, .Interrupt),
                OP.CLV => self.clearSetBit(.Clear, .Overflow),
                OP.SEC => self.clearSetBit(.Set, .Carry),
                OP.SED => self.clearSetBit(.Set, .Decimal),
                OP.SEI => self.clearSetBit(.Set, .Interrupt),

                OP.ASL_ACC => self.displaceReg(.ShiftLeft, A),
                OP.ASL_ZP => self.displaceMem(.ShiftLeft, .ZeroPage),
                OP.ASL_ZPX => self.displaceMem(.ShiftLeft, .ZeroPageX),
                OP.ASL_ABS => self.displaceMem(.ShiftLeft, .Absolute),
                OP.ASL_ABSX => self.displaceMem(.ShiftLeft, .AbsoluteX),

                OP.LSR_ACC => self.displaceReg(.ShiftRight, A),
                OP.LSR_ZP => self.displaceMem(.ShiftRight, .ZeroPage),
                OP.LSR_ZPX => self.displaceMem(.ShiftRight, .ZeroPageX),
                OP.LSR_ABS => self.displaceMem(.ShiftRight, .Absolute),
                OP.LSR_ABSX => self.displaceMem(.ShiftRight, .AbsoluteX),

                OP.ROL_ACC => self.displaceReg(.RotateLeft, A),
                OP.ROL_ZP => self.displaceMem(.RotateLeft, .ZeroPage),
                OP.ROL_ZPX => self.displaceMem(.RotateLeft, .ZeroPageX),
                OP.ROL_ABS => self.displaceMem(.RotateLeft, .Absolute),
                OP.ROL_ABSX => self.displaceMem(.RotateLeft, .AbsoluteX),

                OP.ROR_ACC => self.displaceReg(.RotateRight, A),
                OP.ROR_ZP => self.displaceMem(.RotateRight, .ZeroPage),
                OP.ROR_ZPX => self.displaceMem(.RotateRight, .ZeroPageX),
                OP.ROR_ABS => self.displaceMem(.RotateRight, .Absolute),
                OP.ROR_ABSX => self.displaceMem(.RotateRight, .AbsoluteX),

                OP.BCC => self.branchOnBit(Status.Name.Carry, .Clear),
                OP.BCS => self.branchOnBit(Status.Name.Carry, .Set),
                OP.BEQ => self.branchOnBit(Status.Name.Zero, .Set),
                OP.BMI => self.branchOnBit(Status.Name.Negative, .Set),
                OP.BNE => self.branchOnBit(Status.Name.Zero, .Clear),
                OP.BPL => self.branchOnBit(Status.Name.Negative, .Clear),
                OP.BVC => self.branchOnBit(Status.Name.Overflow, .Clear),
                OP.BVS => self.branchOnBit(Status.Name.Overflow, .Set),

                OP.JMP_ABS => self.jumpToAddress(.Absolute, false),
                OP.JMP_IND => self.jumpToAddress(.Indirect, false),
                OP.JSR_ABS => self.jumpToAddress(.Absolute, true),
                OP.RTS => self.returnToAddress(),

                OP.BRK => self.forceInterrupt(),
                OP.RTI => self.returnFromInterrupt(),

                OP.NOP => self.tick(),
            }
        }
        const used = self.ticks - start;
        return used;
    }

    fn fetch(self: *CPU, register: usize, mode: AddressingMode) void {
        const address = self.computeAddress(mode, false);
        const value = self.readByte(address);
        self.regs[register] = value;
        self.setNZ(self.regs[register]);
    }

    fn store(self: *CPU, register: usize, mode: AddressingMode) void {
        const address = self.computeAddress(mode, false);
        self.writeByte(address, self.regs[register]);
    }

    fn transfer(self: *CPU, source: usize, target: usize, flags: bool) void {
        self.regs[target] = self.regs[source];
        if (flags) {
            self.setNZ(self.regs[target]);
        }
        self.tick();
    }

    fn pushRegisterToStack(self: *CPU, register: usize) void {
        const pushed = self.regs[register];
        self.pushByte(pushed);
        self.tick();
    }

    fn popRegisterFromStack(self: *CPU, register: usize) void {
        const popped = self.popByte(true);
        self.regs[register] = popped;
        self.setNZ(popped);
        self.tick();
    }

    fn pushPSToStack(self: *CPU) void {
        var pushed = self.PS;
        pushed.bits.B = 1;
        pushed.bits.U = 1;
        self.pushByte(pushed.byte);
        self.tick();
    }

    fn popPSFromStack(self: *CPU, doTick: bool) void {
        const popped = self.popByte(doTick);
        self.PS.byte = popped;
        self.PS.bits.B = 0;
        self.PS.bits.U = 0;
        if (doTick) {
            self.tick();
        }
    }

    fn pushByte(self: *CPU, value: Type.Byte) void {
        const address = @as(Type.Word, STACK_BASE) + self.regs[SP];
        self.writeByte(address, value);
        self.regs[SP] -%= 1;
    }

    fn pushWord(self: *CPU, value: Type.Word) void {
        const hi = @as(Type.Word, value >> 8);
        const lo = @as(Type.Word, value & 0xFF);
        self.pushByte(@intCast(Type.Byte, hi));
        self.pushByte(@intCast(Type.Byte, lo));
    }

    fn popByte(self: *CPU, doTick: bool) Type.Byte {
        self.regs[SP] +%= 1;
        const address = @as(Type.Word, STACK_BASE) + self.regs[SP];
        const value = self.readByte(address);
        if (doTick) {
            self.tick();
        }
        return value;
    }

    fn popWord(self: *CPU) Type.Word {
        const lo = @as(Type.Word, self.popByte(false));
        self.tick();
        const hi = @as(Type.Word, self.popByte(false)) << 8;
        self.tick();
        const value = hi | lo;
        return value;
    }

    fn bitOp(self: *CPU, op: BitOp, register: usize, mode: AddressingMode) void {
        const address = self.computeAddress(mode, false);
        const value = self.readByte(address);
        const result = switch (op) {
            .And, .Bit => self.regs[register] & value,
            .InclusiveOr => self.regs[register] | value,
            .ExclusiveOr => self.regs[register] ^ value,
        };
        if (op == .Bit) {
            self.PS.bits.N = if ((value & 0b10000000) > 0) 1 else 0;
            self.PS.bits.V = if ((value & 0b01000000) > 0) 1 else 0;
            self.PS.bits.Z = if ((result | 0b00000000) > 0) 0 else 1;
        } else {
            self.setNZ(result);
            self.regs[register] = result;
        }
    }

    fn numOp(self: *CPU, op: NumOp, register: usize, mode: AddressingMode) void {
        const address = self.computeAddress(mode, false);
        const value = self.readByte(address);
        switch (op) {
            .Add => self.addWithCarry(register, value),
            .Subtract => self.addWithCarry(register, ~value),
        }
    }

    fn addWithCarry(self: *CPU, register: usize, value: Type.Byte) void {
        const reg: Type.Word = self.regs[register];
        const val: Type.Word = value;
        const car: Type.Word = if (self.PS.bits.C > 0) 1 else 0;
        const result = reg + val + car;
        self.PS.bits.C = if (result > 0xFF) 1 else 0;
        self.PS.bits.V = if (((~(reg ^ val)) & (result ^ val) & 0x80) > 0) 1 else 0;
        // std.debug.print("{x} + {x} + {x} = {x}\n", .{ reg, val, car, result });
        self.regs[register] = @intCast(Type.Byte, result & 0xFF);
        self.setNZ(self.regs[register]);
    }

    fn incDecReg(self: *CPU, op: IncDecOp, register: usize) void {
        switch (op) {
            .Increment => self.regs[register] +%= 1,
            .Decrement => self.regs[register] -%= 1,
        }
        self.setNZ(self.regs[register]);
        self.tick();
    }

    fn incDecMem(self: *CPU, op: IncDecOp, mode: AddressingMode) void {
        const address = self.computeAddress(mode, true);
        var value = self.readByte(address);
        switch (op) {
            .Increment => value +%= 1,
            .Decrement => value -%= 1,
        }
        self.writeByte(address, value);
        self.setNZ(value);
        self.tick();
    }

    fn clearSetBit(self: *CPU, op: ClearSet, bit: Status.Name) void {
        const value: Type.Bit = switch (op) {
            .Clear => 0,
            .Set => 1,
        };
        self.PS.setBitByName(bit, value);
        self.tick();
    }

    fn displaceReg(self: *CPU, op: DisplaceOp, register: usize) void {
        self.regs[register] = self.displaceByte(op, self.regs[register]);
    }

    fn displaceMem(self: *CPU, op: DisplaceOp, mode: AddressingMode) void {
        const address = self.computeAddress(mode, true);
        var value = self.readByte(address);
        value = self.displaceByte(op, value);
        self.writeByte(address, value);
    }

    fn displaceByte(self: *CPU, op: DisplaceOp, value: Type.Byte) Type.Byte {
        var displaced = value;
        switch (op) {
            .ShiftLeft => {
                self.PS.bits.C = if ((displaced & 0b10000000) > 0) 1 else 0;
                displaced <<= 1;
            },
            .ShiftRight => {
                self.PS.bits.C = if ((displaced & 0b00000001) > 0) 1 else 0;
                displaced >>= 1;
            },
            .RotateLeft => {
                const oldC = self.PS.bits.C;
                self.PS.bits.C = if ((displaced & 0b10000000) > 0) 1 else 0;
                displaced <<= 1;
                if (oldC > 0) {
                    displaced |= 1;
                }
            },
            .RotateRight => {
                const oldC = self.PS.bits.C;
                self.PS.bits.C = if ((displaced & 0b00000001) > 0) 1 else 0;
                displaced >>= 1;
                if (oldC > 0) {
                    displaced |= 0b10000000;
                }
            },
        }
        self.setNZ(displaced);
        self.tick();
        return displaced;
    }

    fn branchOnBit(self: *CPU, bit: Status.Name, state: ClearSet) void {
        const current = self.PS.getBitByName(bit);
        const delta = @bitCast(i8, self.readByte(self.PC));
        self.PC += 1;
        const branch = switch (state) {
            .Clear => current == 0,
            .Set => current > 0,
        };
        if (branch) {
            self.tick();
            const newPC = @bitCast(Type.Word, @bitCast(i16, self.PC) + delta);
            if (!samePage(self.PC, newPC)) {
                self.tick();
            }
            self.PC = newPC;
        }
    }

    fn jumpToAddress(self: *CPU, mode: AddressingMode, pushReturn: bool) void {
        // http://www.obelisk.me.uk/6502/reference.html#JMP
        // An original 6502 does not correctly fetch the target address if
        // the indirect vector falls on a page boundary (e.g. $xxFF where xx is
        // any value from $00 to $FF).  In this case fetches the LSB from $xxFF
        // as expected but takes the MSB from $xx00.  This is fixed in some
        // later chips like the 65SC02 so for compatibility always ensure the
        // indirect vector is not at the end of the page.
        const address = self.computeAddress(mode, false);
        if (pushReturn) {
            self.pushWord(self.PC - 1);
            self.tick();
        }
        self.PC = address;
    }

    fn returnToAddress(self: *CPU) void {
        self.PC = self.popWord() + 1;
        self.tick();
    }

    fn compareRegister(self: *CPU, register: usize, mode: AddressingMode) void {
        const address = self.computeAddress(mode, false);
        const value = self.readByte(address);
        self.PS.bits.C = if (self.regs[register] >= value) 1 else 0;
        self.PS.bits.Z = if (self.regs[register] == value) 1 else 0;
        self.PS.bits.N = if (self.regs[register] < value) 1 else 0;
    }

    fn forceInterrupt(self: *CPU) void {
        const interrupt_vector: Type.Word = 0xFFFE;
        self.pushWord(self.PC + 1);
        self.pushPSToStack();
        self.PC = self.readWord(interrupt_vector);
        self.PS.bits.B = 1;
        self.PS.bits.I = 1;
    }

    fn returnFromInterrupt(self: *CPU) void {
        self.popPSFromStack(false);
        self.PC = self.popWord();
    }

    fn computeAddress(self: *CPU, mode: AddressingMode, alwaysUseExtra: bool) Type.Word {
        const address = switch (mode) {
            .Immediate => blk: {
                const address = self.PC;
                self.PC += 1;
                break :blk address;
            },
            .ZeroPage => blk: {
                const address = @as(Type.Word, self.readByte(self.PC));
                self.PC += 1;
                break :blk address;
            },
            .ZeroPageX => blk: {
                var address = @as(Type.Word, self.readByte(self.PC));
                self.PC += 1;
                address +%= self.regs[X];
                address &= 0xFF;
                self.tick();
                break :blk address;
            },
            .ZeroPageY => blk: {
                var address = @as(Type.Word, self.readByte(self.PC));
                self.PC += 1;
                address +%= self.regs[Y];
                address &= 0xFF;
                self.tick();
                break :blk address;
            },
            .Absolute => blk: {
                const address = self.readWord(self.PC);
                self.PC += 2;
                break :blk address;
            },
            .AbsoluteX => blk: {
                const initial = self.readWord(self.PC);
                self.PC += 2;
                const final = initial + self.regs[X];
                if (alwaysUseExtra or !samePage(initial, final)) {
                    self.tick();
                }
                break :blk final;
            },
            .AbsoluteY => blk: {
                const initial = self.readWord(self.PC);
                self.PC += 2;
                const final = initial + self.regs[Y];
                if (alwaysUseExtra or !samePage(initial, final)) {
                    self.tick();
                }
                break :blk final;
            },
            .Indirect => blk: {
                const initial = self.readWord(self.PC);
                self.PC += 2;
                const address = self.readWord(initial);
                break :blk address;
            },
            .IndirectX => blk: {
                var address = @as(Type.Word, self.readByte(self.PC));
                self.PC += 1;
                address +%= self.regs[X];
                address &= 0xFF;
                self.tick();
                const final = self.readWord(address);
                break :blk final;
            },
            .IndirectY => blk: {
                const address = @as(Type.Word, self.readByte(self.PC));
                self.PC += 1;
                const initial = self.readWord(address);
                const final = initial + self.regs[Y];
                if (alwaysUseExtra or !samePage(initial, final)) {
                    self.tick();
                }
                break :blk final;
            },
        };
        return address;
    }

    fn readByte(self: *CPU, address: Type.Word) Type.Byte {
        const value = self.memory.data[address];
        self.tick();
        return value;
    }

    fn readWord(self: *CPU, address: Type.Word) Type.Word {
        const lo = @as(Type.Word, self.readByte(address + 0));
        const hi = @as(Type.Word, self.readByte(address + 1)) << 8;
        const value = hi | lo;
        return value;
    }

    fn writeByte(self: *CPU, address: Type.Word, value: Type.Byte) void {
        self.memory.data[address] = value;
        self.tick();
    }

    fn tick(self: *CPU) void {
        self.ticks += 1;
    }

    fn setNZ(self: *CPU, value: Type.Byte) void {
        self.PS.bits.N = if ((value & 0b10000000) > 0) 1 else 0;
        self.PS.bits.Z = if ((value | 0b00000000) > 0) 0 else 1;
    }

    fn samePage(p1: Type.Word, p2: Type.Word) bool {
        // Sometimes adding something to an address will incur in an extra tick
        // ONLY when that caused the address to cross onto another page.
        return (p1 & 0xFF00) == (p2 & 0xFF00);
    }
};

// =========================================================

const TEST_ADDRESS = 0x4433;

test "create CPU" {
    var cpu = CPU.init();
    try testing.expect(cpu.PC == CPU.INITIAL_ADDRESS);
    cpu.reset(TEST_ADDRESS);
    try testing.expect(cpu.PC == TEST_ADDRESS);
}

fn test_load_register(cpu: *CPU, register: usize, address: Type.Word, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
        N: Type.Bit,
        Z: Type.Bit,
    };
    const data = [_]Data{
        .{ .v = 0x11, .N = 0, .Z = 0 },
        .{ .v = 0xF0, .N = 1, .Z = 0 },
        .{ .v = 0x00, .N = 0, .Z = 1 },
    };
    for (data) |d| {
        cpu.PC = TEST_ADDRESS;
        cpu.memory.data[address] = d.v; // put value in memory address
        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers
        cpu.regs[register] = 0; // set desired register to 0
        cpu.PS.byte = 0; // set PS to 0
        const used = cpu.run(ticks);

        try testing.expect(used == ticks);
        try testing.expect(cpu.regs[register] == d.v); // got correct value in register?
        try testing.expect(cpu.PS.bits.N == d.N); // got correct N bit?
        try testing.expect(cpu.PS.bits.Z == d.Z); // got correct Z bit?

        // other bits didn't change?
        try testing.expect(cpu.PS.bits.C == prevPS.bits.C);
        try testing.expect(cpu.PS.bits.I == prevPS.bits.I);
        try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
        try testing.expect(cpu.PS.bits.B == prevPS.bits.B);
        try testing.expect(cpu.PS.bits.V == prevPS.bits.V);

        // registers either got set or didn't change?
        try testing.expect(register == CPU.A or cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(register == CPU.X or cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(register == CPU.Y or cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
        try testing.expect(register == CPU.SP or cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
    }
}

fn test_save_register(cpu: *CPU, register: usize, address: Type.Word, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
    };
    const data = [_]Data{
        .{ .v = 0x11 },
        .{ .v = 0xF0 },
        .{ .v = 0x00 },
    };
    for (data) |d| {
        cpu.PC = TEST_ADDRESS;
        cpu.regs[register] = d.v; // put value in register
        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers
        cpu.memory.data[address] = 0; // set desired address to 0
        cpu.PS.byte = 0; // set PS to 0
        const used = cpu.run(ticks);

        try testing.expect(used == ticks);
        try testing.expect(cpu.memory.data[address] == d.v); // got correct value in memory?
        try testing.expect(cpu.PS.byte == prevPS.byte); // PS didn't change?

        // registers didn't change?
        try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
    }
}

fn test_transfer_register(cpu: *CPU, source: usize, target: usize, flags: bool, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
        N: Type.Bit,
        Z: Type.Bit,
    };
    const data = [_]Data{
        .{ .v = 0x11, .N = 0, .Z = 0 },
        .{ .v = 0xF0, .N = 1, .Z = 0 },
        .{ .v = 0x00, .N = 0, .Z = 1 },
    };
    for (data) |d| {
        cpu.PC = TEST_ADDRESS;
        cpu.regs[source] = d.v; // put value in source register
        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers
        cpu.regs[target] = 0; // set target register to 0
        cpu.PS.byte = 0; // set PS to 0
        const used = cpu.run(ticks);

        try testing.expect(used == ticks);
        try testing.expect(cpu.regs[target] == d.v); // got correct value in target registry?
        if (flags) {
            try testing.expect(cpu.PS.bits.N == d.N); // got correct N bit?
            try testing.expect(cpu.PS.bits.Z == d.Z); // got correct Z bit?
        } else {
            try testing.expect(cpu.PS.bits.N == prevPS.bits.N);
            try testing.expect(cpu.PS.bits.Z == prevPS.bits.Z);
        }

        // other bits didn't change?
        try testing.expect(cpu.PS.bits.C == prevPS.bits.C);
        try testing.expect(cpu.PS.bits.I == prevPS.bits.I);
        try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
        try testing.expect(cpu.PS.bits.B == prevPS.bits.B);
        try testing.expect(cpu.PS.bits.V == prevPS.bits.V);

        // registers either got set or didn't change?
        try testing.expect(target == CPU.A or cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(target == CPU.X or cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(target == CPU.Y or cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
        try testing.expect(target == CPU.SP or cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
    }
}

fn test_push_register(cpu: *CPU, register: usize, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
    };
    const data = [_]Data{
        .{ .v = 0x11 },
        .{ .v = 0xF0 },
        .{ .v = 0x00 },
    };
    for (data) |d| {
        cpu.PC = TEST_ADDRESS;
        cpu.PS.byte = 0; // set PS to 0
        cpu.regs[register] = d.v; // put value in register
        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers
        const used = cpu.run(ticks);

        try testing.expect(used == ticks);
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP] - 1); // did SP move?
        const address = @as(Type.Word, CPU.STACK_BASE) + prevRegs[CPU.SP];
        try testing.expect(cpu.memory.data[address] == d.v); // did the value get pushed?

        // none of the bits changed?
        try testing.expect(cpu.PS.byte == prevPS.byte);

        // registers didn't change?
        try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
    }
}

fn test_push_status(cpu: *CPU, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
    };
    const data = [_]Data{
        .{ .v = 0x11 },
        .{ .v = 0xF0 },
        .{ .v = 0x00 },
    };
    for (data) |d| {
        cpu.PC = TEST_ADDRESS;
        cpu.PS.byte = d.v; // put value in PS
        var pushed = cpu.PS;
        pushed.setBitByName(.Break, 1);
        pushed.setBitByName(.UNUSED, 1);

        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers
        const used = cpu.run(ticks);

        try testing.expect(used == ticks);
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP] - 1); // did SP move?
        const address = @as(Type.Word, CPU.STACK_BASE) + prevRegs[CPU.SP];
        try testing.expect(cpu.memory.data[address] == pushed.byte); // did the value get pushed?

        // none of the bits changed?
        try testing.expect(cpu.PS.byte == prevPS.byte);

        // registers didn't change?
        try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
    }
}

fn test_pop_register(cpu: *CPU, register: usize, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
        N: Type.Bit,
        Z: Type.Bit,
    };
    const data = [_]Data{
        .{ .v = 0x11, .N = 0, .Z = 0 },
        .{ .v = 0xF0, .N = 1, .Z = 0 },
        .{ .v = 0x00, .N = 0, .Z = 1 },
    };
    var SP: Type.Byte = 0xFF;
    for (data) |d| {
        const address = @as(Type.Word, CPU.STACK_BASE) + SP;
        cpu.memory.data[address] = d.v; // set value in stack
        SP -%= 1;
    }
    cpu.regs[CPU.SP] = SP;
    for (data) |_, p| {
        const pos = data.len - 1 - p;
        cpu.PC = TEST_ADDRESS;
        cpu.PS.byte = 0; // set PS to 0
        cpu.regs[register] = 0; // set register to 0
        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers
        const used = cpu.run(ticks);

        try testing.expect(used == ticks);
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP] + 1); // did SP move?
        const address = @as(Type.Word, CPU.STACK_BASE) + prevRegs[CPU.SP];
        _ = address;
        try testing.expect(cpu.regs[register] == data[pos].v); // did the value get popped?

        try testing.expect(cpu.PS.bits.N == data[pos].N); // got correct N bit?
        try testing.expect(cpu.PS.bits.Z == data[pos].Z); // got correct Z bit?
        // other bits didn't change?
        try testing.expect(cpu.PS.bits.C == prevPS.bits.C);
        try testing.expect(cpu.PS.bits.I == prevPS.bits.I);
        try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
        try testing.expect(cpu.PS.bits.B == prevPS.bits.B);
        try testing.expect(cpu.PS.bits.V == prevPS.bits.V);

        // registers didn't change?
        try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
    }
    try testing.expect(cpu.regs[CPU.SP] == 0xFF);
}

fn test_pop_status(cpu: *CPU, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
        N: Type.Bit,
        Z: Type.Bit,
    };
    const data = [_]Data{
        .{ .v = 0x11, .N = 0, .Z = 0 },
        .{ .v = 0xF0, .N = 1, .Z = 0 },
        .{ .v = 0x00, .N = 0, .Z = 1 },
    };
    var SP: Type.Byte = 0xFF;
    for (data) |d| {
        const address = @as(Type.Word, CPU.STACK_BASE) + SP;
        cpu.memory.data[address] = d.v; // set value in stack
        SP -%= 1;
    }
    cpu.regs[CPU.SP] = SP;
    for (data) |_, p| {
        const pos = data.len - 1 - p;
        cpu.PC = TEST_ADDRESS;
        cpu.PS.byte = 0; // set PS to 0
        const prevPS = cpu.PS; // remember PS

        _ = prevPS;

        const prevRegs = cpu.regs; // remember registers
        const used = cpu.run(ticks);
        try testing.expect(used == ticks);

        var popped = cpu.PS;
        popped.byte = data[pos].v; // did the value get popped?
        popped.setBitByName(.Break, 0);
        popped.setBitByName(.UNUSED, 0);
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP] + 1); // did SP move?
        const address = @as(Type.Word, CPU.STACK_BASE) + prevRegs[CPU.SP];
        _ = address;
        try testing.expect(cpu.PS.byte == popped.byte); // did the value get popped?

        // registers didn't change?
        try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
    }
    try testing.expect(cpu.regs[CPU.SP] == 0xFF);
}

fn test_bitop_register(cpu: *CPU, op: CPU.BitOp, register: usize, address: Type.Word, ticks: u32) !void {
    const Data = struct {
        m: Type.Byte,
        r: Type.Byte,
    };
    const data = [_]Data{
        .{
            .m = 0b00110001,
            .r = 0b10101010,
        },
        .{
            .m = 0b01010101,
            .r = 0b10101010,
        },
        .{
            .m = 0b10110011,
            .r = 0b10101010,
        },
    };
    for (data) |d| {
        cpu.PC = TEST_ADDRESS;
        cpu.memory.data[address] = d.m; // put value in memory address
        const prevPS = cpu.PS; // remember PS

        const prevRegs = cpu.regs; // remember registers
        cpu.regs[register] = d.r; // set desired register
        cpu.PS.byte = 0; // set PS to 0

        const afterR: Type.Byte = switch (op) {
            .And, .Bit => d.m & d.r,
            .InclusiveOr => d.m | d.r,
            .ExclusiveOr => d.m ^ d.r,
        };

        const used = cpu.run(ticks);
        try testing.expect(used == ticks);

        const afterZ: Type.Bit = if ((afterR | 0b00000000) > 0) 0 else 1;
        try testing.expect(cpu.PS.bits.Z == afterZ); // got correct N bit?

        if (op == .Bit) {
            const afterN: Type.Bit = if ((d.m & 0b10000000) > 0) 1 else 0;
            const afterV: Type.Bit = if ((d.m & 0b01000000) > 0) 1 else 0;
            try testing.expect(cpu.PS.bits.N == afterN); // got correct N bit?
            try testing.expect(cpu.PS.bits.V == afterV); // got correct V bit?
        } else {
            const afterN: Type.Bit = if ((afterR & 0b10000000) > 0) 1 else 0;
            try testing.expect(cpu.PS.bits.N == afterN); // got correct N bit?
            try testing.expect(cpu.regs[register] == afterR); // got correct after value in register?
        }

        // other bits didn't change?
        try testing.expect(cpu.PS.bits.C == prevPS.bits.C);
        try testing.expect(cpu.PS.bits.I == prevPS.bits.I);
        try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
        try testing.expect(cpu.PS.bits.B == prevPS.bits.B);

        // registers either got set or didn't change?
        try testing.expect(register == CPU.A or cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(register == CPU.X or cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(register == CPU.Y or cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
        try testing.expect(register == CPU.SP or cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
    }
}

fn test_numop_register(cpu: *CPU, op: CPU.NumOp, register: usize, address: Type.Word, ticks: u32) !void {
    const Data = struct {
        oC: Type.Bit,
        oR: Type.Byte,
        oM: Type.Byte,
        aR: Type.Byte,
        sR: Type.Byte,
        aC: Type.Bit,
        aZ: Type.Bit,
        aV: Type.Bit,
        aN: Type.Bit,
        sC: Type.Bit,
        sZ: Type.Bit,
        sV: Type.Bit,
        sN: Type.Bit,
    };
    const data = [_]Data{
        .{
            .oC = 0,
            .oR = 0b00000000,
            .oM = 0b00000000,
            .aR = 0b00000000,
            .sR = 0b11111111,
            .aC = 0,
            .aZ = 1,
            .aV = 0,
            .aN = 0,
            .sC = 0,
            .sZ = 0,
            .sV = 0,
            .sN = 1,
        },
        .{
            .oC = 1,
            .oR = 0b00000000,
            .oM = 0b00000000,
            .aR = 0b00000001,
            .sR = 0b00000000,
            .aC = 0,
            .aZ = 0,
            .aV = 0,
            .aN = 0,
            .sC = 1,
            .sZ = 1,
            .sV = 0,
            .sN = 0,
        },
        .{
            .oC = 0,
            .oR = 0b00100100, // 36
            .oM = 0b00001101, // 13
            .aR = 0b00110001, // 49
            .sR = 0b00010110, // 22
            .aC = 0,
            .aZ = 0,
            .aV = 0,
            .aN = 0,
            .sC = 1,
            .sZ = 0,
            .sV = 0,
            .sN = 0,
        },
        .{
            .oC = 1,
            .oR = 0b00100100, // 36
            .oM = 0b00001101, // 13
            .aR = 0b00110010, // 50
            .sR = 0b00010111, // 23
            .aC = 0,
            .aZ = 0,
            .aV = 0,
            .aN = 0,
            .sC = 1,
            .sZ = 0,
            .sV = 0,
            .sN = 0,
        },
        .{
            .oC = 0,
            .oR = 0b01000001, //  65
            .oM = 0b01000000, //  64
            .aR = 0b10000001, // 129
            .sR = 0b00000000, //   0
            .aC = 0,
            .aZ = 0,
            .aV = 1,
            .aN = 1,
            .sC = 1,
            .sZ = 1,
            .sV = 0,
            .sN = 0,
        },
        .{
            .oC = 1,
            .oR = 0b01000010, //  66
            .oM = 0b01000000, //  64
            .aR = 0b10000011, // 131
            .sR = 0b00000010, //   2
            .aC = 0,
            .aZ = 0,
            .aV = 1,
            .aN = 1,
            .sC = 1,
            .sZ = 0,
            .sV = 0,
            .sN = 0,
        },
        .{
            .oC = 0,
            .oR = 0b10000001, // 129
            .oM = 0b10000001, // 129
            .aR = 0b00000010, //   2
            .sR = 0b11111111, //  -1
            .aC = 1,
            .aZ = 0,
            .aV = 1,
            .aN = 0,
            .sC = 0,
            .sZ = 0,
            .sV = 0,
            .sN = 1,
        },
        .{
            .oC = 1,
            .oR = 0b10000001, // 129
            .oM = 0b10000001, // 129
            .aR = 0b00000011, //   3
            .sR = 0b00000000, //   0
            .aC = 1,
            .aZ = 0,
            .aV = 1,
            .aN = 0,
            .sC = 1,
            .sZ = 1,
            .sV = 0,
            .sN = 0,
        },
        .{
            .oC = 0,
            .oR = 0b00001001, //  9
            .oM = 0b11111100, // -4
            .aR = 0b00000101, //  5
            .sR = 0b00001100, // 12
            .aC = 1,
            .aZ = 0,
            .aV = 0,
            .aN = 0,
            .sC = 0,
            .sZ = 0,
            .sV = 0,
            .sN = 0,
        },
        .{
            .oC = 1,
            .oR = 0b00001001, //  9
            .oM = 0b11111100, // -4
            .aR = 0b00000110, //  6
            .sR = 0b00001101, // 13
            .aC = 1,
            .aZ = 0,
            .aV = 0,
            .aN = 0,
            .sC = 0,
            .sZ = 0,
            .sV = 0,
            .sN = 0,
        },
    };
    for (data) |d| {
        cpu.PC = TEST_ADDRESS;
        cpu.memory.data[address] = d.oM; // put value in memory address
        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers
        cpu.regs[register] = d.oR; // set desired register
        cpu.PS.bits.C = d.oC; // set carry

        const used = cpu.run(ticks);
        try testing.expect(used == ticks);

        switch (op) {
            .Add => {
                try testing.expect(cpu.regs[register] == d.aR); // got correct result?
                try testing.expect(cpu.PS.bits.C == d.aC); // got correct C bit?
                try testing.expect(cpu.PS.bits.Z == d.aZ); // got correct Z bit?
                try testing.expect(cpu.PS.bits.V == d.aV); // got correct V bit?
                try testing.expect(cpu.PS.bits.N == d.aN); // got correct N bit?
            },
            .Subtract => {
                try testing.expect(cpu.regs[register] == d.sR); // got correct result?
                try testing.expect(cpu.PS.bits.C == d.sC); // got correct C bit?
                try testing.expect(cpu.PS.bits.Z == d.sZ); // got correct Z bit?
                try testing.expect(cpu.PS.bits.V == d.sV); // got correct V bit?
                try testing.expect(cpu.PS.bits.N == d.sN); // got correct N bit?
            },
        }

        // other bits didn't change?
        try testing.expect(cpu.PS.bits.I == prevPS.bits.I);
        try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
        try testing.expect(cpu.PS.bits.B == prevPS.bits.B);

        // registers either got set or didn't change?
        try testing.expect(register == CPU.A or cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(register == CPU.X or cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(register == CPU.Y or cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
        try testing.expect(register == CPU.SP or cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
    }
}

fn test_compare_register(cpu: *CPU, register: usize, address: Type.Word, ticks: u32) !void {
    const Data = struct {
        R: Type.Byte,
        M: Type.Byte,
        C: Type.Bit,
        Z: Type.Bit,
        N: Type.Bit,
    };
    const data = [_]Data{
        .{
            .R = 0x43,
            .M = 0x33,
            .C = 1,
            .Z = 0,
            .N = 0,
        },
        .{
            .R = 0x33,
            .M = 0x33,
            .C = 1,
            .Z = 1,
            .N = 0,
        },
        .{
            .R = 0x33,
            .M = 0x43,
            .C = 0,
            .Z = 0,
            .N = 1,
        },
    };
    for (data) |d| {
        cpu.PC = TEST_ADDRESS;
        cpu.memory.data[address] = d.M; // put value in memory address
        cpu.regs[register] = d.R; // set desired register
        cpu.PS.byte = 0;
        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers

        const used = cpu.run(ticks);
        try testing.expect(used == ticks);

        try testing.expect(cpu.PS.bits.C == d.C); // got correct C bit?
        try testing.expect(cpu.PS.bits.Z == d.Z); // got correct Z bit?
        try testing.expect(cpu.PS.bits.N == d.N); // got correct N bit?

        // other bits didn't change?
        try testing.expect(cpu.PS.bits.I == prevPS.bits.I);
        try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
        try testing.expect(cpu.PS.bits.B == prevPS.bits.B);
        try testing.expect(cpu.PS.bits.V == prevPS.bits.V);

        // registers didn't change?
        try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
    }
}

fn test_inc_dec(cpu: *CPU, op: CPU.IncDecOp, register: usize, address: Type.Word, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
        o: CPU.IncDecOp,
        e: Type.Byte,
        N: Type.Bit,
        Z: Type.Bit,
    };
    const data = [_]Data{
        .{ .v = 0x11, .o = .Increment, .e = 0x12, .N = 0, .Z = 0 },
        .{ .v = 0x11, .o = .Decrement, .e = 0x10, .N = 0, .Z = 0 },
        .{ .v = 0xF0, .o = .Increment, .e = 0xF1, .N = 1, .Z = 0 },
        .{ .v = 0xF0, .o = .Decrement, .e = 0xEF, .N = 1, .Z = 0 },
        .{ .v = 0x00, .o = .Increment, .e = 0x01, .N = 0, .Z = 0 },
        .{ .v = 0x00, .o = .Decrement, .e = 0xFF, .N = 1, .Z = 0 },
        .{ .v = 0x01, .o = .Increment, .e = 0x02, .N = 0, .Z = 0 },
        .{ .v = 0x01, .o = .Decrement, .e = 0x00, .N = 0, .Z = 1 },
        .{ .v = 0xFF, .o = .Increment, .e = 0x00, .N = 0, .Z = 1 },
        .{ .v = 0xFF, .o = .Decrement, .e = 0xFE, .N = 1, .Z = 0 },
    };
    for (data) |d| {
        if (op != d.o) {
            continue;
        }
        cpu.PC = TEST_ADDRESS;
        if (address == 0) {
            cpu.regs[register] = d.v; // set desired register
        } else {
            cpu.memory.data[address] = d.v; // set desired address
        }

        const prevPS = cpu.PS; // remember PS

        const prevRegs = cpu.regs; // remember registers

        _ = prevRegs;

        cpu.PS.byte = 0; // set PS to 0

        const used = cpu.run(ticks);
        try testing.expect(used == ticks);

        if (address == 0) {
            try testing.expect(cpu.regs[register] == d.e);
        } else {
            try testing.expect(cpu.memory.data[address] == d.e);
        }
        try testing.expect(cpu.PS.bits.N == d.N); // got correct N bit?
        try testing.expect(cpu.PS.bits.Z == d.Z); // got correct Z bit?

        // other bits didn't change?
        try testing.expect(cpu.PS.bits.C == prevPS.bits.C);
        try testing.expect(cpu.PS.bits.I == prevPS.bits.I);
        try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
        try testing.expect(cpu.PS.bits.B == prevPS.bits.B);
        try testing.expect(cpu.PS.bits.V == prevPS.bits.V);
    }
}

fn test_set_bit(cpu: *CPU, op: CPU.ClearSet, bit: Status.Name, ticks: u32) !void {
    cpu.PC = TEST_ADDRESS;
    switch (op) {
        .Clear => cpu.PS.setBitByName(bit, 1),
        .Set => cpu.PS.setBitByName(bit, 0),
    }

    const prevPS = cpu.PS; // remember PS

    const prevRegs = cpu.regs; // remember registers

    _ = prevRegs;

    const used = cpu.run(ticks);
    try testing.expect(used == ticks);

    switch (op) {
        .Clear => try testing.expect(cpu.PS.getBitByName(bit) == 0),
        .Set => try testing.expect(cpu.PS.getBitByName(bit) == 1),
    }

    // other bits didn't change?
    try testing.expect(bit == .Carry or cpu.PS.bits.C == prevPS.bits.C);
    try testing.expect(bit == .Zero or cpu.PS.bits.Z == prevPS.bits.Z);
    try testing.expect(bit == .Interrupt or cpu.PS.bits.I == prevPS.bits.I);
    try testing.expect(bit == .Decimal or cpu.PS.bits.D == prevPS.bits.D);
    try testing.expect(bit == .Break or cpu.PS.bits.B == prevPS.bits.B);
    try testing.expect(bit == .Overflow or cpu.PS.bits.V == prevPS.bits.V);
    try testing.expect(bit == .Negative or cpu.PS.bits.N == prevPS.bits.N);
}

fn test_branch_bit(cpu: *CPU, bit: Status.Name, state: CPU.ClearSet, ticks: u32) !void {
    const Data = struct {
        b: Type.Bit,
        s: CPU.ClearSet,
        d: u8,
        x: u32,
    };
    const data = [_]Data{
        .{ .b = 0, .s = .Clear, .d = 0x10, .x = 1 },
        .{ .b = 0, .s = .Set, .d = 0x10, .x = 0 },
        .{ .b = 1, .s = .Clear, .d = 0x10, .x = 0 },
        .{ .b = 1, .s = .Set, .d = 0x10, .x = 1 },
        .{ .b = 0, .s = .Clear, .d = 0x81, .x = 2 },
        .{ .b = 0, .s = .Set, .d = 0x81, .x = 0 },
        .{ .b = 1, .s = .Clear, .d = 0x81, .x = 0 },
        .{ .b = 1, .s = .Set, .d = 0x81, .x = 2 },
    };
    for (data) |d| {
        if (state != d.s) {
            continue;
        }
        cpu.PC = TEST_ADDRESS;
        cpu.memory.data[TEST_ADDRESS + 1] = d.d;
        cpu.PS.setBitByName(bit, d.b);

        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers

        const expected = ticks + d.x;
        const used = cpu.run(expected);
        try testing.expect(used == expected);
        var newPC: Type.Word = TEST_ADDRESS + 2;
        if (d.x > 0) {
            newPC = @bitCast(Type.Word, @bitCast(i16, newPC) + @bitCast(i8, d.d));
        }
        try testing.expect(cpu.PC == newPC);
        try testing.expect(cpu.PS.byte == prevPS.byte);
        try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
        try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
        try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
    }
}

fn test_jmp(cpu: *CPU, address: Type.Word, pushReturn: bool, ticks: u32) !void {
    cpu.PC = TEST_ADDRESS;
    const prevPS = cpu.PS; // remember PS
    const prevRegs = cpu.regs; // remember registers

    const used = cpu.run(ticks);
    try testing.expect(used == ticks);
    try testing.expect(cpu.PC == address);
    try testing.expect(cpu.PS.byte == prevPS.byte);
    try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
    try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
    try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
    if (pushReturn) {
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP] - 2);
    } else {
        try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
    }
}

fn test_rts(cpu: *CPU) !void {
    cpu.PC = TEST_ADDRESS;
    const prevPS = cpu.PS; // remember PS
    const prevRegs = cpu.regs; // remember registers

    const ticks: u32 = 6 + 6;
    const used = cpu.run(ticks);
    try testing.expect(used == ticks);
    try testing.expect(cpu.PC == TEST_ADDRESS + 3);
    try testing.expect(cpu.PS.byte == prevPS.byte);
    try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
    try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
    try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
    try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
}

fn test_brk(cpu: *CPU) !void {
    cpu.PC = TEST_ADDRESS;
    cpu.memory.data[0xFFFE] = 0x37;
    cpu.memory.data[0xFFFF] = 0x41;
    cpu.PS.bits.B = 0;
    const prevPS = cpu.PS; // remember PS
    const prevRegs = cpu.regs; // remember registers

    const ticks: u32 = 7;
    const used = cpu.run(ticks);
    try testing.expect(used == ticks);
    try testing.expect(cpu.PC == 0x4137);
    try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP] - 3);
    try testing.expect(cpu.PS.bits.B == 1);
    try testing.expect(cpu.PS.bits.I == 1);

    // other bits didn't change?
    try testing.expect(cpu.PS.bits.C == prevPS.bits.C);
    try testing.expect(cpu.PS.bits.Z == prevPS.bits.Z);
    try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
    try testing.expect(cpu.PS.bits.V == prevPS.bits.V);
    try testing.expect(cpu.PS.bits.N == prevPS.bits.N);

    try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
    try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
    try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
}

fn test_rti(cpu: *CPU) !void {
    cpu.PC = TEST_ADDRESS;
    cpu.memory.data[0xFFFE] = 0x37;
    cpu.memory.data[0xFFFF] = 0x41;
    cpu.memory.data[0x4137] = 0x40;
    const prevPS = cpu.PS; // remember PS
    const prevRegs = cpu.regs; // remember registers

    const ticks: u32 = 7 + 6;
    const used = cpu.run(ticks);
    try testing.expect(used == ticks);
    try testing.expect(cpu.PC == TEST_ADDRESS + 2);
    try testing.expect(cpu.PS.byte == prevPS.byte);
    try testing.expect(cpu.regs[CPU.A] == prevRegs[CPU.A]);
    try testing.expect(cpu.regs[CPU.X] == prevRegs[CPU.X]);
    try testing.expect(cpu.regs[CPU.Y] == prevRegs[CPU.Y]);
    try testing.expect(cpu.regs[CPU.SP] == prevRegs[CPU.SP]);
}

fn test_displace_bit(cpu: *CPU, op: CPU.DisplaceOp, register: usize, address: Type.Word, ticks: u32) !void {
    const Data = struct {
        v: Type.Byte,
        o: CPU.DisplaceOp,
        e: Type.Byte,
        oC: Type.Bit,
        nC: Type.Bit,
    };
    const data = [_]Data{
        .{ .v = 0b00000000, .o = .ShiftLeft, .e = 0b00000000, .oC = 0, .nC = 0 },
        .{ .v = 0b00000001, .o = .ShiftLeft, .e = 0b00000010, .oC = 0, .nC = 0 },
        .{ .v = 0b10000001, .o = .ShiftLeft, .e = 0b00000010, .oC = 0, .nC = 1 },
        .{ .v = 0b00000000, .o = .ShiftRight, .e = 0b00000000, .oC = 0, .nC = 0 },
        .{ .v = 0b00000001, .o = .ShiftRight, .e = 0b00000000, .oC = 0, .nC = 1 },
        .{ .v = 0b10000001, .o = .ShiftRight, .e = 0b01000000, .oC = 0, .nC = 1 },
        .{ .v = 0b00000000, .o = .RotateLeft, .e = 0b00000000, .oC = 0, .nC = 0 },
        .{ .v = 0b00000000, .o = .RotateLeft, .e = 0b00000001, .oC = 1, .nC = 0 },
        .{ .v = 0b00000001, .o = .RotateLeft, .e = 0b00000010, .oC = 0, .nC = 0 },
        .{ .v = 0b00000001, .o = .RotateLeft, .e = 0b00000011, .oC = 1, .nC = 0 },
        .{ .v = 0b10000001, .o = .RotateLeft, .e = 0b00000010, .oC = 0, .nC = 1 },
        .{ .v = 0b10000001, .o = .RotateLeft, .e = 0b00000011, .oC = 1, .nC = 1 },
        .{ .v = 0b00000000, .o = .RotateRight, .e = 0b00000000, .oC = 0, .nC = 0 },
        .{ .v = 0b00000000, .o = .RotateRight, .e = 0b10000000, .oC = 1, .nC = 0 },
        .{ .v = 0b00000001, .o = .RotateRight, .e = 0b00000000, .oC = 0, .nC = 1 },
        .{ .v = 0b00000001, .o = .RotateRight, .e = 0b10000000, .oC = 1, .nC = 1 },
        .{ .v = 0b10000001, .o = .RotateRight, .e = 0b01000000, .oC = 0, .nC = 1 },
        .{ .v = 0b10000001, .o = .RotateRight, .e = 0b11000000, .oC = 1, .nC = 1 },
    };
    for (data) |d| {
        if (op != d.o) {
            continue;
        }
        cpu.PC = TEST_ADDRESS;
        if (address == 0) {
            cpu.regs[register] = d.v;
        } else {
            cpu.memory.data[address] = d.v;
        }
        cpu.PS.bits.C = d.oC;

        const prevPS = cpu.PS; // remember PS
        const prevRegs = cpu.regs; // remember registers

        _ = prevRegs;

        const used = cpu.run(ticks);
        try testing.expect(used == ticks);

        if (address == 0) {
            cpu.regs[register] = d.e;
        } else {
            cpu.memory.data[address] = d.e;
        }
        try testing.expect(cpu.PS.bits.C == d.nC);

        const nZ: Type.Bit = if ((d.e | 0b00000000) > 0) 0 else 1;
        const nN: Type.Bit = if ((d.e & 0b10000000) > 0) 1 else 0;
        try testing.expect(cpu.PS.bits.Z == nZ);
        try testing.expect(cpu.PS.bits.N == nN);

        // other bits didn't change?
        try testing.expect(cpu.PS.bits.I == prevPS.bits.I);
        try testing.expect(cpu.PS.bits.D == prevPS.bits.D);
        try testing.expect(cpu.PS.bits.B == prevPS.bits.B);
        try testing.expect(cpu.PS.bits.V == prevPS.bits.V);
    }
}

// LDA tests

test "run LDA_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xA9;
    try test_load_register(&cpu, CPU.A, TEST_ADDRESS + 1, 2);
}

test "run LDA_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xA5;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_load_register(&cpu, CPU.A, 0x0011, 3);
}

test "run LDA_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB5;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_load_register(&cpu, CPU.A, 0x0011 + 7, 4);
}

test "run LDA_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xAD;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.A, 0x8311, 4);
}

test "run LDA_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xBD;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.A, 0x8311 + 7, 4);
}

test "run LDA_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB9;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.A, 0x8311 + 7, 4);
}

test "run LDA_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xBD;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.A, 0x8311 + 0xFE, 5);
}

test "run LDA_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB9;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.A, 0x8311 + 0xFE, 5);
}

test "run LDA_INDX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 4;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xA1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x20;
    cpu.memory.data[0x20 + 4 + 0] = 0x74;
    cpu.memory.data[0x20 + 4 + 1] = 0x20;
    try test_load_register(&cpu, CPU.A, 0x2074, 6);
}

test "run LDA_INDY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0x10;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_load_register(&cpu, CPU.A, 0x4028 + 0x10, 5);
}

test "run LDA_INDY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_load_register(&cpu, CPU.A, 0x4028 + 0xFE, 6);
}

// LDX tests

test "run LDX_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xA2;
    try test_load_register(&cpu, CPU.X, TEST_ADDRESS + 1, 2);
}

test "run LDX_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xA6;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_load_register(&cpu, CPU.X, 0x0011, 3);
}

test "run LDX_ZPY" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB6;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_load_register(&cpu, CPU.X, 0x0011 + 7, 4);
}

test "run LDX_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xAE;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.X, 0x8311, 4);
}

test "run LDX_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xBE;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.X, 0x8311 + 7, 4);
}

test "run LDX_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xBE;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.X, 0x8311 + 0xFE, 5);
}

// LDY tests

test "run LDY_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xA0;
    try test_load_register(&cpu, CPU.Y, TEST_ADDRESS + 1, 2);
}

test "run LDY_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xA4;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_load_register(&cpu, CPU.Y, 0x0011, 3);
}

test "run LDY_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB4;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_load_register(&cpu, CPU.Y, 0x0011 + 7, 4);
}

test "run LDY_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xAC;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.Y, 0x8311, 4);
}

test "run LDY_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xBC;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.Y, 0x8311 + 7, 4);
}

test "run LDY_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xBC;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_load_register(&cpu, CPU.Y, 0x8311 + 0xFE, 5);
}

// STA tests

test "run STA_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x85;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_save_register(&cpu, CPU.A, 0x0011, 3);
}

test "run STA_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x95;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_save_register(&cpu, CPU.A, 0x0011 + 7, 4);
}

test "run STA_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x8D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_save_register(&cpu, CPU.A, 0x8311, 4);
}

test "run STA_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x9D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_save_register(&cpu, CPU.A, 0x8311 + 7, 4);
}

test "run STA_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x99;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_save_register(&cpu, CPU.A, 0x8311 + 7, 4);
}

test "run STA_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x9D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_save_register(&cpu, CPU.A, 0x8311 + 0xFE, 5);
}

test "run STA_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x99;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_save_register(&cpu, CPU.A, 0x8311 + 0xFE, 5);
}

test "run STA_INDX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 4;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x81;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x20;
    cpu.memory.data[0x20 + 4 + 0] = 0x74;
    cpu.memory.data[0x20 + 4 + 1] = 0x20;
    try test_save_register(&cpu, CPU.A, 0x2074, 6);
}

test "run STA_INDY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0x10;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x91;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_save_register(&cpu, CPU.A, 0x4028 + 0x10, 5);
}

test "run STA_INDY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x91;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_save_register(&cpu, CPU.A, 0x4028 + 0xFE, 6);
}

// STX tests

test "run STX_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x86;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_save_register(&cpu, CPU.X, 0x0011, 3);
}

test "run STX_ZPY" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x96;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_save_register(&cpu, CPU.X, 0x0011 + 7, 4);
}

test "run STX_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x8E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_save_register(&cpu, CPU.X, 0x8311, 4);
}

// STY tests

test "run STY_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x84;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_save_register(&cpu, CPU.Y, 0x0011, 3);
}

test "run STY_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x94;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_save_register(&cpu, CPU.Y, 0x0011 + 7, 4);
}

test "run STY_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x8C;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_save_register(&cpu, CPU.Y, 0x8311, 4);
}

// TRANSFER tests

test "run TAX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xAA;
    try test_transfer_register(&cpu, CPU.A, CPU.X, true, 2);
}

test "run TAY" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xA8;
    try test_transfer_register(&cpu, CPU.A, CPU.Y, true, 2);
}

test "run TXA" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x8A;
    try test_transfer_register(&cpu, CPU.X, CPU.A, true, 2);
}

test "run TYA" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x98;
    try test_transfer_register(&cpu, CPU.Y, CPU.A, true, 2);
}

test "run TSX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xBA;
    try test_transfer_register(&cpu, CPU.SP, CPU.X, true, 2);
}

test "run TXS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x9A;
    try test_transfer_register(&cpu, CPU.X, CPU.SP, false, 2);
}

// push

test "run PHA" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x48;
    try test_push_register(&cpu, CPU.A, 3);
}

test "run PHP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x08;
    try test_push_status(&cpu, 3);
}

// pop

test "run PLA" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x68;
    try test_pop_register(&cpu, CPU.A, 4);
}

test "run PLP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x28;
    try test_pop_status(&cpu, 4);
}

// AND tests

test "run AND_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x29;
    try test_bitop_register(&cpu, .And, CPU.A, TEST_ADDRESS + 1, 2);
}

test "run AND_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x25;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_bitop_register(&cpu, .And, CPU.A, 0x0011, 3);
}

test "run AND_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x35;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_bitop_register(&cpu, .And, CPU.A, 0x0011 + 7, 4);
}

test "run AND_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x2D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .And, CPU.A, 0x8311, 4);
}

test "run AND_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x3D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .And, CPU.A, 0x8311 + 7, 4);
}

test "run AND_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x39;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .And, CPU.A, 0x8311 + 7, 4);
}

test "run AND_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x3D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .And, CPU.A, 0x8311 + 0xFE, 5);
}

test "run AND_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x39;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .And, CPU.A, 0x8311 + 0xFE, 5);
}

test "run AND_INDX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 4;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x21;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x20;
    cpu.memory.data[0x20 + 4 + 0] = 0x74;
    cpu.memory.data[0x20 + 4 + 1] = 0x20;
    try test_bitop_register(&cpu, .And, CPU.A, 0x2074, 6);
}

test "run AND_INDY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0x10;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x31;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_bitop_register(&cpu, .And, CPU.A, 0x4028 + 0x10, 5);
}

test "run AND_INDY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x31;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_bitop_register(&cpu, .And, CPU.A, 0x4028 + 0xFE, 6);
}

// EOR tests

test "run EOR_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x49;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, TEST_ADDRESS + 1, 2);
}

test "run EOR_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x45;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x0011, 3);
}

test "run EOR_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x55;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x0011 + 7, 4);
}

test "run EOR_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x4D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x8311, 4);
}

test "run EOR_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x5D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x8311 + 7, 4);
}

test "run EOR_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x59;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x8311 + 7, 4);
}

test "run EOR_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x5D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x8311 + 0xFE, 5);
}

test "run EOR_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x59;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x8311 + 0xFE, 5);
}

test "run EOR_INDX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 4;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x41;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x20;
    cpu.memory.data[0x20 + 4 + 0] = 0x74;
    cpu.memory.data[0x20 + 4 + 1] = 0x20;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x2074, 6);
}

test "run EOR_INDY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0x10;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x51;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x4028 + 0x10, 5);
}

test "run EOR_INDY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x51;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_bitop_register(&cpu, .ExclusiveOr, CPU.A, 0x4028 + 0xFE, 6);
}

// ORA tests

test "run ORA_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x09;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, TEST_ADDRESS + 1, 2);
}

test "run ORA_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x05;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x0011, 3);
}

test "run ORA_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x15;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x0011 + 7, 4);
}

test "run ORA_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x0D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x8311, 4);
}

test "run ORA_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x1D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x8311 + 7, 4);
}

test "run ORA_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x19;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x8311 + 7, 4);
}

test "run ORA_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x1D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x8311 + 0xFE, 5);
}

test "run ORA_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x19;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x8311 + 0xFE, 5);
}

test "run ORA_INDX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 4;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x01;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x20;
    cpu.memory.data[0x20 + 4 + 0] = 0x74;
    cpu.memory.data[0x20 + 4 + 1] = 0x20;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x2074, 6);
}

test "run ORA_INDY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0x10;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x4028 + 0x10, 5);
}

test "run ORA_INDY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_bitop_register(&cpu, .InclusiveOr, CPU.A, 0x4028 + 0xFE, 6);
}

// BIT tests

test "run BIT_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x24;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_bitop_register(&cpu, .Bit, CPU.A, 0x0011, 3);
}

test "run BIT_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x2C;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_bitop_register(&cpu, .Bit, CPU.A, 0x8311, 4);
}

// ADC tests

test "run ADC_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x69;
    try test_numop_register(&cpu, .Add, CPU.A, TEST_ADDRESS + 1, 2);
}

test "run ADC_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x65;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_numop_register(&cpu, .Add, CPU.A, 0x0011, 3);
}

test "run ADC_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x75;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_numop_register(&cpu, .Add, CPU.A, 0x0011 + 7, 4);
}

test "run ADC_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x6D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Add, CPU.A, 0x8311, 4);
}

test "run ADC_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x7D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Add, CPU.A, 0x8311 + 7, 4);
}

test "run ADC_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x79;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Add, CPU.A, 0x8311 + 7, 4);
}

test "run ADC_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x7D;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Add, CPU.A, 0x8311 + 0xFE, 5);
}

test "run ADC_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x79;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Add, CPU.A, 0x8311 + 0xFE, 5);
}

test "run ADC_INDX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 4;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x61;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x20;
    cpu.memory.data[0x20 + 4 + 0] = 0x74;
    cpu.memory.data[0x20 + 4 + 1] = 0x20;
    try test_numop_register(&cpu, .Add, CPU.A, 0x2074, 6);
}

test "run ADC_INDY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0x10;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x71;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_numop_register(&cpu, .Add, CPU.A, 0x4028 + 0x10, 5);
}

test "run ADC_INDY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x71;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_numop_register(&cpu, .Add, CPU.A, 0x4028 + 0xFE, 6);
}

// SBC tests

test "run SBC_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xE9;
    try test_numop_register(&cpu, .Subtract, CPU.A, TEST_ADDRESS + 1, 2);
}

test "run SBC_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xE5;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x0011, 3);
}

test "run SBC_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xF5;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x0011 + 7, 4);
}

test "run SBC_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xED;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x8311, 4);
}

test "run SBC_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xFD;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x8311 + 7, 4);
}

test "run SBC_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xF9;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x8311 + 7, 4);
}

test "run SBC_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xFD;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x8311 + 0xFE, 5);
}

test "run SBC_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xF9;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x8311 + 0xFE, 5);
}

test "run SBC_INDX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 4;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xE1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x20;
    cpu.memory.data[0x20 + 4 + 0] = 0x74;
    cpu.memory.data[0x20 + 4 + 1] = 0x20;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x2074, 6);
}

test "run SBC_INDY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0x10;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xF1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x4028 + 0x10, 5);
}

test "run SBC_INDY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xF1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_numop_register(&cpu, .Subtract, CPU.A, 0x4028 + 0xFE, 6);
}

// CMP tests

test "run CMP_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xC9;
    try test_compare_register(&cpu, CPU.A, TEST_ADDRESS + 1, 2);
}

test "run CMP_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xC5;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_compare_register(&cpu, CPU.A, 0x0011, 3);
}

test "run CMP_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xD5;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_compare_register(&cpu, CPU.A, 0x0011 + 7, 4);
}

test "run CMP_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xCD;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_compare_register(&cpu, CPU.A, 0x8311, 4);
}

test "run CMP_ABSX same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xDD;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_compare_register(&cpu, CPU.A, 0x8311 + 7, 4);
}

test "run CMP_ABSY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xD9;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_compare_register(&cpu, CPU.A, 0x8311 + 7, 4);
}

test "run CMP_ABSX cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xDD;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_compare_register(&cpu, CPU.A, 0x8311 + 0xFE, 5);
}

test "run CMP_ABSY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xD9;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_compare_register(&cpu, CPU.A, 0x8311 + 0xFE, 5);
}

test "run CMP_INDX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 4;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xC1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x20;
    cpu.memory.data[0x20 + 4 + 0] = 0x74;
    cpu.memory.data[0x20 + 4 + 1] = 0x20;
    try test_compare_register(&cpu, CPU.A, 0x2074, 6);
}

test "run CMP_INDY same page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0x10;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xD1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_compare_register(&cpu, CPU.A, 0x4028 + 0x10, 5);
}

test "run CMP_INDY cross page" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.Y] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xD1;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x86;
    cpu.memory.data[0x86 + 0] = 0x28;
    cpu.memory.data[0x86 + 1] = 0x40;
    try test_compare_register(&cpu, CPU.A, 0x4028 + 0xFE, 6);
}

// CPX tests

test "run CPX_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xE0;
    try test_compare_register(&cpu, CPU.X, TEST_ADDRESS + 1, 2);
}

test "run CPX_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xE4;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_compare_register(&cpu, CPU.X, 0x0011, 3);
}

test "run CPX_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xEC;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_compare_register(&cpu, CPU.X, 0x8311, 4);
}

// CPY tests

test "run CPY_IMM" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xC0;
    try test_compare_register(&cpu, CPU.Y, TEST_ADDRESS + 1, 2);
}

test "run CPY_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xC4;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_compare_register(&cpu, CPU.Y, 0x0011, 3);
}

test "run CPY_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xCC;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_compare_register(&cpu, CPU.Y, 0x8311, 4);
}

// INC tests

test "run INC_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xE6;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_inc_dec(&cpu, .Increment, 0, 0x0011, 5);
}

test "run INC_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xF6;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_inc_dec(&cpu, .Increment, 0, 0x0011 + 7, 6);
}

test "run INC_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xEE;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_inc_dec(&cpu, .Increment, 0, 0x8311, 6);
}

test "run INC_ABSX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xFE;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_inc_dec(&cpu, .Increment, 0, 0x8311 + 7, 7);
}

test "run INCX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xE8;
    try test_inc_dec(&cpu, .Increment, CPU.X, 0, 2);
}

test "run INCY" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xC8;
    try test_inc_dec(&cpu, .Increment, CPU.Y, 0, 2);
}

// DEC tests

test "run DEC_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xC6;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_inc_dec(&cpu, .Decrement, 0, 0x0011, 5);
}

test "run DEC_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xD6;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_inc_dec(&cpu, .Decrement, 0, 0x0011 + 7, 6);
}

test "run DEC_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xCE;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_inc_dec(&cpu, .Decrement, 0, 0x8311, 6);
}

test "run DEC_ABSX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0xDE;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_inc_dec(&cpu, .Decrement, 0, 0x8311 + 7, 7);
}

test "run DECX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xCA;
    try test_inc_dec(&cpu, .Decrement, CPU.X, 0, 2);
}

test "run DECY" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x88;
    try test_inc_dec(&cpu, .Decrement, CPU.Y, 0, 2);
}

// CLR / SET tests

test "run CLC" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x18;
    try test_set_bit(&cpu, .Clear, .Carry, 2);
}

test "run CLD" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xD8;
    try test_set_bit(&cpu, .Clear, .Decimal, 2);
}

test "run CLI" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x58;
    try test_set_bit(&cpu, .Clear, .Interrupt, 2);
}

test "run CLV" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB8;
    try test_set_bit(&cpu, .Clear, .Overflow, 2);
}

test "run SEC" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x38;
    try test_set_bit(&cpu, .Set, .Carry, 2);
}

test "run SED" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xF8;
    try test_set_bit(&cpu, .Set, .Decimal, 2);
}

test "run SEI" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x78;
    try test_set_bit(&cpu, .Set, .Interrupt, 2);
}

// ASL tests

test "run ASL_ACC" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x0A;
    try test_displace_bit(&cpu, .ShiftLeft, CPU.A, 0, 2);
}

test "run ASL_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x06;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_displace_bit(&cpu, .ShiftLeft, 0, 0x11, 5);
}

test "run ASL_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x16;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_displace_bit(&cpu, .ShiftLeft, 0, 0x11 + 7, 6);
}

test "run ASL_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x0E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_displace_bit(&cpu, .ShiftLeft, 0, 0x8311, 6);
}

test "run ASL_ABSX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x1E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_displace_bit(&cpu, .ShiftLeft, 0, 0x8311 + 7, 7);
}

// LSR tests

test "run LSR_ACC" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x4A;
    try test_displace_bit(&cpu, .ShiftRight, CPU.A, 0, 2);
}

test "run LSR_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x46;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_displace_bit(&cpu, .ShiftRight, 0, 0x11, 5);
}

test "run LSR_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x56;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_displace_bit(&cpu, .ShiftRight, 0, 0x11 + 7, 6);
}

test "run LSR_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x4E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_displace_bit(&cpu, .ShiftRight, 0, 0x8311, 6);
}

test "run LSR_ABSX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x5E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_displace_bit(&cpu, .ShiftRight, 0, 0x8311 + 7, 7);
}

// ROL tests

test "run ROL_ACC" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x2A;
    try test_displace_bit(&cpu, .RotateLeft, CPU.A, 0, 2);
}

test "run ROL_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x26;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_displace_bit(&cpu, .RotateLeft, 0, 0x11, 5);
}

test "run ROL_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x36;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_displace_bit(&cpu, .RotateLeft, 0, 0x11 + 7, 6);
}

test "run ROL_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x2E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_displace_bit(&cpu, .RotateLeft, 0, 0x8311, 6);
}

test "run ROL_ABSX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x3E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_displace_bit(&cpu, .RotateLeft, 0, 0x8311 + 7, 7);
}

// ROR tests

test "run ROR_ACC" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x6A;
    try test_displace_bit(&cpu, .RotateRight, CPU.A, 0, 2);
}

test "run ROR_ZP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x66;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_displace_bit(&cpu, .RotateRight, 0, 0x11, 5);
}

test "run ROR_ZPX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x76;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    try test_displace_bit(&cpu, .RotateRight, 0, 0x11 + 7, 6);
}

test "run ROR_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x6E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_displace_bit(&cpu, .RotateRight, 0, 0x8311, 6);
}

test "run ROR_ABSX" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.regs[CPU.X] = 7;
    cpu.memory.data[TEST_ADDRESS + 0] = 0x7E;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_displace_bit(&cpu, .RotateRight, 0, 0x8311 + 7, 7);
}

// Bit set / clear tests

test "run BCC" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x90;
    try test_branch_bit(&cpu, .Carry, .Clear, 2);
}

test "run BCS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xB0;
    try test_branch_bit(&cpu, .Carry, .Set, 2);
}

test "run BEQ" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xF0;
    try test_branch_bit(&cpu, .Zero, .Set, 2);
}

test "run BMI" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x30;
    try test_branch_bit(&cpu, .Negative, .Set, 2);
}

test "run BNE" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xD0;
    try test_branch_bit(&cpu, .Zero, .Clear, 2);
}

test "run BPL" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x10;
    try test_branch_bit(&cpu, .Negative, .Clear, 2);
}

test "run BVC" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x50;
    try test_branch_bit(&cpu, .Overflow, .Clear, 2);
}

test "run BVS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x70;
    try test_branch_bit(&cpu, .Overflow, .Set, 2);
}

// JMP tests

test "run JMP_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x4C;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_jmp(&cpu, 0x8311, false, 3);
}

test "run JMP_IND" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x6C;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    cpu.memory.data[0x8311 + 0] = 0x74;
    cpu.memory.data[0x8311 + 1] = 0x20;
    try test_jmp(&cpu, 0x2074, false, 5);
}

// JSR tests

test "run JSR_ABS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x20;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    try test_jmp(&cpu, 0x8311, true, 6);
}

// RTS tests

test "run RTS" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x20;
    cpu.memory.data[TEST_ADDRESS + 1] = 0x11;
    cpu.memory.data[TEST_ADDRESS + 2] = 0x83;
    cpu.memory.data[0x8311] = 0x60;
    try test_rts(&cpu);
}

// BRK tests

test "run BRK" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x00;
    try test_brk(&cpu);
}

// RTI tests

test "run RTI" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0x00;
    try test_rti(&cpu);
}

// NOP tests

test "run NOP" {
    var cpu = CPU.init();
    cpu.reset(TEST_ADDRESS);
    cpu.memory.data[TEST_ADDRESS + 0] = 0xEA;
    const used = cpu.run(2);
    try testing.expect(used == 2);
}
