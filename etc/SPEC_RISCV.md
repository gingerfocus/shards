# SPEC: RISC-V Interpreter (rv64emu)

## Overview

A profiling RISC-V interpreter that executes RV64G + RVV 1.0 instructions,
records memory access patterns per page, tracks basic block execution counts,
and produces execution traces for the pattern detector.

This is NOT a full cycle-accurate emulator. It is a functional interpreter
with instrumentation hooks for profiling and lowering.

## ISA Support

### MVP (Phase 2)

| Extension | Instructions | Count | Notes |
|-----------|-------------|-------|-------|
| RV64I | lui, auipc, jal, jalr, beq, bne, blt, bge, bltu, bgeu, lb, lh, lw, ld, lbu, lhu, lwu, sb, sh, sw, sd, addi, slti, sltiu, xori, ori, andi, slli, srli, srai, add, sub, sll, slt, sltu, xor, srl, sra, or, and, fence, fence.i, ecall, ebreak, csrrw, csrrs, csrrc, csrrwi, csrrsi, csrrci | ~50 | All base integer |
| RV64M | mul, mulh, mulhsu, mulhu, div, divu, rem, remu | 8 | Multiply/divide |
| RV64F | flw, fsw, fmadd.s, fmsub.s, fnmsub.s, fnmadd.s, fadd.s, fsub.s, fmul.s, fdiv.s, fsqrt.s, fsgnj.s, fsgnjn.s, fsgnjx.s, fmin.s, fmax.s, fcvt.w.s, fcvt.wu.s, fmv.x.w, feq.s, flt.s, fle.s, fclass.s, fcvt.s.w, fcvt.s.wu, fmv.w.x | 26 | Single-precision float |
| RV64D | fld, fsd, fmadd.d, fmsub.d, ..., fcvt.d.s, fcvt.s.d, ... | 26 | Double-precision float |
| RVV subset | vsetvli, vle.v, vse.v, vadd.vv, vsub.vv, vmul.vv, vfmul.vv | 7 | Vector ops needed for pattern recognition |
| **Total MVP** | | **~117** | |

### Post-MVP (Phase 6)

- Full RVV 1.0 (all 300+ instructions)
- RV64A (atomics)
- RV64C (compressed)
- Privileged spec (full CSR set, traps, interrupts)

## Instruction Decoding

All 32-bit RISC-V instructions follow a consistent encoding:

```
[31:25]   funct7
[24:20]   rs2
[19:15]   rs1
[14:12]   funct3
[11:7]    rd
[6:0]     opcode
```

```rust
struct DecodedInstruction {
    opcode:   u8,         // [6:0]
    rd:       Option<u8>,  // [11:7], None for S/B-type
    rs1:      Option<u8>,  // [19:15], None for U/J-type
    rs2:      Option<u8>,  // [24:20], None for I-type
    funct3:   u8,         // [14:12]
    funct7:   u8,         // [31:25]
    funct12:  u16,        // [31:20] (for RVV instructions)
    imm_i:    i64,        // sign-extended [31:20]
    imm_s:    i64,        // [31:25][11:7]
    imm_b:    i64,        // [31][7][30:25][11:8]
    imm_u:    i64,        // [31:12] << 12
    imm_j:    i64,        // [31][19:12][20][30:21]
    vd:       Option<u8>,  // RVV destination register
    vs1:      Option<u8>,  // RVV source 1 register
    vs2:      Option<u8>,  // RVV source 2 register
    vm:       bool,        // RVV mask bit
}

impl DecodedInstruction {
    fn decode(raw: u32) -> Self {
        let opcode = (raw & 0x7F) as u8;
        let rd     = ((raw >> 7)  & 0x1F) as u8;
        let funct3 = ((raw >> 12) & 0x07) as u8;
        let rs1    = ((raw >> 15) & 0x1F) as u8;
        let rs2    = ((raw >> 20) & 0x1F) as u8;
        let funct7 = ((raw >> 25) & 0x7F) as u8;

        DecodedInstruction {
            opcode,
            rd: Some(rd),
            rs1: Some(rs1),
            rs2: Some(rs2),
            funct3,
            funct7,
            imm_i: sign_extend((raw >> 20) as u16 as i64, 12),
            imm_s: sign_extend((((raw >> 25) << 5) | ((raw >> 7) & 0x1F)) as i64, 12),
            imm_b: sign_extend(/* complex bit packing */, 13),
            imm_u: ((raw as i64) & 0xFFFF_F000) as i64,
            imm_j: sign_extend(/* complex bit packing */, 21),
            // ... RVV fields
        }
    }
}
```

## Register File

```rust
struct RegisterFile {
    x:    [u64; 32],      // Integer registers x0-x31
                          // x0 is hardwired to 0 (writes ignored)
    f:    [u64; 32],      // Floating-point registers f0-f31
                          // Stored as u64 bits for both f32 and f64
    v:    [Vec<u8>; 32],  // Vector registers v0-v31
                          // Variable-length byte buffers
    csr:  CsrFile,
}

impl RegisterFile {
    fn read_gpr(&self, reg: u8) -> u64 {
        if reg == 0 { 0 } else { self.x[reg as usize] }
    }

    fn write_gpr(&mut self, reg: u8, value: u64) {
        if reg != 0 { self.x[reg as usize] = value; }
    }

    fn read_fpr(&self, reg: u8) -> u64 {
        self.f[reg as usize]
    }

    fn write_fpr(&mut self, reg: u8, value: u64) {
        self.f[reg as usize] = value;
    }
}
```

## Execution Loop

```rust
struct VM {
    regs:        RegisterFile,
    pc:          u64,
    page_table:  HashMap<u64, PageDescriptor>,
    segments:    Vec<SegmentDescriptor>,
    cycle_count: u64,
    inst_count:  u64,
    tracer:      GlobalTrace,
    scheduler:   ExecutionScheduler,
    checkpoints: CheckpointManager,
}

impl VM {
    fn step(&mut self) -> StepResult {
        // 1. Instruction fetch
        let raw = match self.virtual_fetch(self.pc) {
            Ok(raw) => raw,
            Err(trap) => return StepResult::Trap(trap),
        };
        let dec = DecodedInstruction::decode(raw);

        // 2. Execute
        let result = self.execute(&dec);

        // 3. Update PC
        match &result {
            ExecResult::Sequential => self.pc += 4,
            ExecResult::Branch(target) => self.pc = *target,
            ExecResult::Ecall => return StepResult::Ecall,
            ExecResult::Ebreak => return StepResult::Ebreak,
            ExecResult::Trap(t) => return StepResult::Trap(*t),
        }

        // 4. Tick counters
        self.cycle_count += 1;
        self.inst_count += 1;

        // 5. Maybe checkpoint
        if self.should_checkpoint() {
            self.checkpoints.capture(self);
        }

        // 6. Maybe tier up
        if self.scheduler.should_tier_up(&self.tracer) {
            return StepResult::Lower(self.scheduler.lowering_plan());
        }

        StepResult::Continue
    }

    fn run(&mut self, max_cycles: u64) -> RunResult {
        let start = self.cycle_count;

        while self.cycle_count - start < max_cycles {
            match self.step() {
                StepResult::Continue => continue,
                other => return RunResult {
                    cycles: self.cycle_count - start,
                    result: other,
                    traces: self.tracer.drain(),
                },
            }
        }

        RunResult {
            cycles: max_cycles,
            result: StepResult::Continue,
            traces: self.tracer.drain(),
        }
    }
}

enum StepResult {
    Continue,
    Ecall,
    Ebreak,
    Trap(Trap),
    Lower(LoweringPlan),
}

struct RunResult {
    cycles: u64,
    result: StepResult,
    traces: Vec<InsnTrace>,
}
```

## Instruction Execution (Example Subset)

```rust
fn execute(&mut self, dec: &DecodedInstruction) -> ExecResult {
    let opcode = dec.opcode;

    match opcode {
        // RV64I: Integer Register-Immediate
        0x13 => {  // OP-IMM
            let rd = dec.rd.unwrap();
            let rs1 = self.regs.read_gpr(dec.rs1.unwrap());
            let imm = dec.imm_i;
            let result = match dec.funct3 {
                0x0 => rs1.wrapping_add(imm as u64),         // ADDI
                0x2 => (rs1 as i64 < imm) as u64,             // SLTI
                0x3 => (rs1 < imm as u64) as u64,             // SLTIU
                0x4 => rs1 ^ (imm as u64),                    // XORI
                0x6 => rs1 | (imm as u64),                    // ORI
                0x7 => rs1 & (imm as u64),                    // ANDI
                0x1 => rs1 << ((imm & 0x3F) as u32),          // SLLI
                0x5 => match dec.funct7 {
                    0x00 => rs1 >> ((imm & 0x3F) as u32),    // SRLI
                    0x20 => ((rs1 as i64) >> ((imm & 0x3F) as u32)) as u64, // SRAI
                    _ => return ExecResult::Trap(Trap::IllegalInstruction),
                },
                _ => return ExecResult::Trap(Trap::IllegalInstruction),
            };
            self.regs.write_gpr(rd, result);
            ExecResult::Sequential
        }

        // RV64I: Integer Register-Register
        0x33 => {  // OP
            let rd = dec.rd.unwrap();
            let rs1 = self.regs.read_gpr(dec.rs1.unwrap());
            let rs2 = self.regs.read_gpr(dec.rs2.unwrap());
            let result = match (dec.funct7, dec.funct3) {
                (0x00, 0x0) => rs1.wrapping_add(rs2),         // ADD
                (0x20, 0x0) => rs1.wrapping_sub(rs2),         // SUB
                (0x00, 0x1) => rs1 << (rs2 & 0x3F),           // SLL
                (0x00, 0x2) => (rs1 as i64 < rs2 as i64) as u64, // SLT
                (0x00, 0x3) => (rs1 < rs2) as u64,            // SLTU
                (0x00, 0x4) => rs1 ^ rs2,                     // XOR
                (0x00, 0x5) => rs1 >> (rs2 & 0x3F),           // SRL
                (0x20, 0x5) => ((rs1 as i64) >> (rs2 as i64 as u32)) as u64, // SRA
                (0x00, 0x6) => rs1 | rs2,                     // OR
                (0x00, 0x7) => rs1 & rs2,                     // AND
                _ => return ExecResult::Trap(Trap::IllegalInstruction),
            };
            self.regs.write_gpr(rd, result);
            ExecResult::Sequential
        }

        // Load
        0x03 => {
            let rd = dec.rd.unwrap();
            let addr = self.regs.read_gpr(dec.rs1.unwrap()).wrapping_add(dec.imm_i as u64);
            let result = match dec.funct3 {
                0x0 => self.virtual_load(addr, 1)? as i8 as i64 as u64,  // LB
                0x1 => self.virtual_load(addr, 2)? as i16 as i64 as u64, // LH
                0x2 => self.virtual_load(addr, 4)? as i32 as i64 as u64, // LW
                0x3 => self.virtual_load(addr, 8)?,                      // LD
                0x4 => self.virtual_load(addr, 1)?,                      // LBU
                0x5 => self.virtual_load(addr, 2)?,                      // LHU
                0x6 => self.virtual_load(addr, 4)?,                      // LWU
                _ => return ExecResult::Trap(Trap::IllegalInstruction),
            };
            self.regs.write_gpr(rd, result);
            ExecResult::Sequential
        }

        // Store
        0x23 => {
            let addr = self.regs.read_gpr(dec.rs1.unwrap()).wrapping_add(dec.imm_s as u64);
            let value = self.regs.read_gpr(dec.rs2.unwrap());
            match dec.funct3 {
                0x0 => self.virtual_store(addr, 1, value)?,  // SB
                0x1 => self.virtual_store(addr, 2, value)?,  // SH
                0x2 => self.virtual_store(addr, 4, value)?,  // SW
                0x3 => self.virtual_store(addr, 8, value)?,  // SD
                _ => return ExecResult::Trap(Trap::IllegalInstruction),
            }
            ExecResult::Sequential
        }

        // Branches
        0x63 => {
            let rs1 = self.regs.read_gpr(dec.rs1.unwrap());
            let rs2 = self.regs.read_gpr(dec.rs2.unwrap());
            let taken = match dec.funct3 {
                0x0 => rs1 == rs2,                                 // BEQ
                0x1 => rs1 != rs2,                                 // BNE
                0x4 => (rs1 as i64) < (rs2 as i64),                // BLT
                0x5 => (rs1 as i64) >= (rs2 as i64),               // BGE
                0x6 => rs1 < rs2,                                  // BLTU
                0x7 => rs1 >= rs2,                                 // BGEU
                _ => return ExecResult::Trap(Trap::IllegalInstruction),
            };
            if taken {
                ExecResult::Branch(self.pc.wrapping_add(dec.imm_b as u64))
            } else {
                ExecResult::Sequential
            }
        }

        // System (ecall/ebreak/CSR)
        0x73 => match dec.funct3 {
            0x0 => match dec.imm_i {
                0x000 => ExecResult::Ecall,
                0x001 => ExecResult::Ebreak,
                _ => ExecResult::Trap(Trap::IllegalInstruction),
            },
            _ => { /* CSR ops */ ExecResult::Sequential }
        },

        _ => ExecResult::Trap(Trap::IllegalInstruction),
    }
}

enum ExecResult {
    Sequential,
    Branch(u64),
    Ecall,
    Ebreak,
    Trap(Trap),
}

enum Trap {
    IllegalInstruction,
    PageFault { addr: u64, cause: &'static str },
    MisalignedAddress(u64),
    UnsupportedSize(u8),
}
```

## RVV Subset Implementation

For the MVP, RVV instructions are decoded and executed in scalar form but
with vector-aware tracing:

```rust
// vadd.vv vd, vs1, vs2, vm
fn execute_vadd_vv(&mut self, vd: u8, vs1: u8, vs2: u8, vm: bool, vtype: VType) {
    let sew = vtype.sew_bytes();     // element width (1,2,4,8)
    let vl = self.csr.vl as usize;   // vector length

    for i in 0..vl {
        let offset = i * sew;
        let a = self.regs.read_vreg_element(vs1, offset, sew);
        let b = self.regs.read_vreg_element(vs2, offset, sew);

        if vm || self.regs.read_mask(i) {
            let result = a.wrapping_add(b);
            self.regs.write_vreg_element(vd, offset, sew, result);
        }
    }
}
```

The tracer sees this as N sequential loads from `vs1`, N sequential loads
from `vs2`, and N sequential stores to `vd` — which is the canonical
stride-1 pattern the detector looks for.

## System Call Interface

The VM provides a minimal set of syscalls via `ecall`:

| Syscall | Number | Description |
|---------|--------|-------------|
| exit | 93 | Terminate with exit code |
| write | 64 | Write to file descriptor |
| read | 63 | Read from file descriptor |
| brk | 214 | Set program break (heap boundary) |
| mmap | 222 | Allocate new pages |
| munmap | 215 | Free pages |
| open | 1024 | Open a file |
| close | 57 | Close a file descriptor |

```rust
fn handle_ecall(&mut self) -> ExecResult {
    let sysno = self.regs.x[17];   // a7
    let a0 = self.regs.x[10];
    let a1 = self.regs.x[11];
    let a2 = self.regs.x[12];

    match sysno {
        93 => ExecResult::Exit(a0 as i32),           // exit
        214 => {                                       // brk
            let new_brk = if a0 == 0 {
                self.heap_top                       // get current brk
            } else {
                self.grow_heap(a0)                  // set new brk
            };
            self.regs.write_gpr(10, new_brk);
            ExecResult::Sequential
        }
        222 => {                                       // mmap
            let addr = self.mmap(a0, a1, a2);
            self.regs.write_gpr(10, addr);
            ExecResult::Sequential
        }
        _ => ExecResult::Trap(Trap::UnsupportedSyscall(sysno)),
    }
}
```
