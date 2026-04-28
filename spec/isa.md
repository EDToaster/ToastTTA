# ToastTTA — Instruction Set Architecture v1.0

ToastTTA is a 16-bit transport-triggered architecture (TTA). Programs are
expressed as parallel data movements ("moves") between registers and functional
unit (FU) ports. Operations occur as a side effect of moves: writing data to a
*triggering port* of an FU starts a computation; the result becomes available
at an *output port* after a known latency.

This contrasts with operation-triggered ISAs (RISC, CISC) where instructions
name an operation and the hardware manages internal data routing. In ToastTTA,
data routing *is* the program; operations follow.

---

## 1. Architectural summary

| Property | Value |
|---|---|
| Data path width | 16 bits |
| Buses (parallel moves per cycle) | 4 |
| Instruction word width | 128 bits |
| Slot width | 32 bits |
| GPRs | 16 × 16-bit |
| Boolean RF | 1 × 1-bit |
| Memory model | Harvard |
| Address width | 16 bits |
| FU latencies (v1.0) | All 1 cycle |
| Predication | Per-move guard, single predicate, both polarities |
| I/O | Memory-mapped |

---

## 2. Memory model

### 2.1 Address spaces

ToastTTA uses a Harvard architecture. Instruction memory (I-mem) and data
memory (D-mem) are separate, both addressed by 16-bit pointers.

| Space | Size | Purpose |
|-------|------|---------|
| I-mem | 64 K instruction words (1024 KB) | Holds 128-bit instruction words. Address `n` selects the *n*-th instruction word. |
| D-mem | 64 K data words (128 KB) | Holds 16-bit data words. Address `n` selects the *n*-th 16-bit word. |

Both memories are word-addressable. Byte addressing is not supported in v1.0.

### 2.2 D-memory layout

```
0x0000 .. 0x7FFF    32 KW   general data RAM
0x8000 .. 0xFEFF    27 KW   reserved / future use
0xFF00 .. 0xFFFF    256  W  memory-mapped I/O
```

Behavior of accesses to the reserved region is implementation-defined.

### 2.3 Memory-mapped I/O

Reads from and writes to addresses in the MMIO range are dispatched to
peripheral handlers (or, in the emulator, to runtime functions) rather than
D-memory.

| Address | R/W | Function |
|---------|-----|----------|
| `0xFF00` | W | `STDOUT_CHAR` — write low byte as ASCII to stdout |
| `0xFF01` | W | `STDOUT_INT` — write value as signed decimal integer to stdout |
| `0xFF02` | W | `STDOUT_HEX` — write value as `0x%04X` to stdout |
| `0xFF10` | R | `STDIN_CHAR` — read one byte from stdin (returns 0 if no input available) |
| `0xFF20` | R | `CYCLE_LO` — low 16 bits of cycle counter |
| `0xFF21` | R | `CYCLE_HI` — high 16 bits of cycle counter |
| `0xFFFE` | W | `HALT` — terminate execution; written value is exit code |

All other MMIO addresses are reserved.

---

## 3. Architectural state

### 3.1 General-purpose register file (GPR)

16 registers, each 16 bits wide: `r0`, `r1`, …, `r15`. All are general-purpose;
no register has a hardware-fixed role. The ABI may assign roles by convention.

### 3.2 Boolean register file (BRF)

One register, 1 bit wide: `p0`. Used as a guard predicate for conditional
execution (§6).

### 3.3 Functional-unit output ports

Each FU has one or more output ports that hold its most recent result. These
ports are architecturally visible and addressable as sources:

| Port | Width | Holds |
|------|-------|-------|
| `ALU.R` | 16 bits | Most recent ALU arithmetic result |
| `ALU.P` | 1 bit | Most recent ALU compare result |
| `LSU.R` | 16 bits | Most recent LSU load result |
| `MUL.R` | 16 bits | Most recent MUL low-product result |

Output ports retain their value until overwritten by a subsequent operation on
the same FU. They behave as architecturally visible 1-deep registers.

### 3.4 Program counter

The program counter (`PC`) lives inside the Global Control Unit (GCU) and is
not directly addressable as a source or destination. It is updated only by GCU
jump triggers (§8.3) and by the implicit increment after each instruction word.

`PC` resets to `0x0000` at startup.

---

## 4. Instruction format

### 4.1 Instruction word

Every instruction word is exactly **128 bits**, divided into 4 equal **slots**
of 32 bits each. Each slot independently specifies one move on its own bus.
Buses are not numbered; the slot's position in the word determines its bus.

```
 ┌─────────── 128-bit instruction word ───────────┐
 │  slot 0   │  slot 1   │  slot 2   │  slot 3    │
 │ (32 bits) │ (32 bits) │ (32 bits) │ (32 bits)  │
 └────────────────────────────────────────────────┘
```

All four slots execute simultaneously in one clock cycle.

### 4.2 Slot encoding

Each 32-bit slot has the following layout (bit 31 is most significant):

```
 ┌──── 32-bit slot ────────────────────────────────────────────┐
 │ [31:30]  guard      (2 bits)                                 │
 │ [29:25]  src.sock   (5 bits)                                 │
 │ [24:9]   src.data   (16 bits)                                │
 │  [8:3]   dst.sock   (6 bits)                                 │
 │  [2:0]   reserved   (3 bits, must be zero)                   │
 └──────────────────────────────────────────────────────────────┘
```

| Field | Width | Description |
|-------|-------|-------------|
| `guard` | 2 | Predicate code (§6.1) |
| `src.sock` | 5 | Source socket selector (32 codes) |
| `src.data` | 16 | Immediate value when `src.sock = IMMEDIATE`; otherwise ignored |
| `dst.sock` | 6 | Destination socket selector (64 codes) |
| `reserved` | 3 | Must be zero in v1.0 |

---

## 5. Sockets

### 5.1 Source sockets (`src.sock`, 5 bits)

| ID | Name | Value yielded by reading this socket |
|----|------|--------------------------------------|
| 0..15 | `GPR_READ_r0` .. `GPR_READ_r15` | Contents of `rN` (16 bits) |
| 16 | `BRF_READ_p0` | `p0`, zero-extended to 16 bits |
| 17 | `ALU_R` | Contents of `ALU.R` |
| 18 | `ALU_P` | `ALU.P`, zero-extended to 16 bits |
| 19 | `LSU_R` | Contents of `LSU.R` |
| 20 | `IMMEDIATE` | `src.data` field (sign-/zero-extension is destination-defined) |
| 21 | `MUL_R` | Contents of `MUL.R` |
| 22..31 | Reserved | Must not appear in valid programs |

### 5.2 Destination sockets (`dst.sock`, 6 bits)

| ID | Name | Effect of write |
|----|------|-----------------|
| 0..15 | `GPR_WRITE_r0` .. `GPR_WRITE_r15` | `rN ← value` |
| 16 | `BRF_WRITE_p0` | `p0 ← (value != 0)` |
| 17 | `ALU_A` | Latch operand register; non-triggering |
| 18 | `ALU_ADD_T` | Trigger: `ALU.R ← ALU_A + value` |
| 19 | `ALU_SUB_T` | Trigger: `ALU.R ← ALU_A − value` |
| 20 | `ALU_AND_T` | Trigger: `ALU.R ← ALU_A & value` |
| 21 | `ALU_OR_T` | Trigger: `ALU.R ← ALU_A \| value` |
| 22 | `ALU_XOR_T` | Trigger: `ALU.R ← ALU_A ^ value` |
| 23 | `ALU_SHL_T` | Trigger: `ALU.R ← ALU_A << (value & 0xF)` |
| 24 | `ALU_SHR_T` | Trigger: `ALU.R ← ALU_A >> (value & 0xF)` (logical) |
| 25 | `ALU_SSHR_T` | Trigger: `ALU.R ← ALU_A >> (value & 0xF)` (arithmetic) |
| 26 | `ALU_EQ_T` | Trigger: `ALU.P ← (ALU_A == value)` |
| 27 | `ALU_NE_T` | Trigger: `ALU.P ← (ALU_A != value)` |
| 28 | `ALU_LT_T` | Trigger: `ALU.P ← (ALU_A < value)` (signed) |
| 29 | `ALU_LE_T` | Trigger: `ALU.P ← (ALU_A <= value)` (signed) |
| 30 | `ALU_GT_T` | Trigger: `ALU.P ← (ALU_A > value)` (signed) |
| 31 | `ALU_GE_T` | Trigger: `ALU.P ← (ALU_A >= value)` (signed) |
| 32 | `LSU_LD_T` | Trigger: `LSU.R ← D-mem[value]` (or MMIO read) |
| 33 | `LSU_ST_A` | Latch store address; non-triggering |
| 34 | `LSU_ST_T` | Trigger: `D-mem[LSU_ST_A] ← value` (or MMIO write) |
| 35 | `GCU_JMP_T` | Trigger: `PC ← value` (takes effect next cycle) |
| 36 | `DISCARD` | No effect; value is consumed and dropped |
| 37 | `MUL_A` | Latch operand register; non-triggering |
| 38 | `MUL_T` | Trigger: `MUL.R ← low 16 bits of (MUL_A × value)` |
| 39..63 | Reserved | Must not appear in valid programs |

---

## 6. Predication and guards

### 6.1 Guard codes

| `guard` | Meaning |
|---------|---------|
| `00` | `always` — move executes unconditionally |
| `01` | `if p0` — move executes if `p0 == 1` |
| `10` | `if !p0` — move executes if `p0 == 0` |
| `11` | `never` — move is squashed (NOP) |

### 6.2 Guard semantics

A move whose guard evaluates false is **squashed**: the destination is not
written. The bus and decoder still process the slot; only the destination's
write-enable is gated off. Side effects normally caused by triggering an FU
do not occur for a squashed move.

The guard predicate is sampled from architectural state at the start of the
cycle (§7.1). A move that writes `p0` in cycle *N* does not affect any guards
in cycle *N*; those guards see the value of `p0` at the start of cycle *N*. The
new value becomes visible to guards in cycle *N+1*.

Conditional branches are simply guarded moves to `GCU_JMP_T`. There is no
separate branch-if-condition instruction.

---

## 7. Execution semantics

### 7.1 Cycle model

Execution proceeds in discrete clock cycles. Each cycle, exactly one
instruction word is fetched from `I-mem[PC]` and executed. All four slots of
the word execute in parallel during that cycle.

Each cycle proceeds in two logical phases:

**Phase 1 — Snapshot.** The current values of all readable architectural state
(GPRs, BRF, FU output ports, PC) are captured. All guard predicates are
evaluated against this snapshot. Each active slot's source value is read from
the snapshot.

**Phase 2 — Apply.** For each active (un-squashed) slot, the destination is
written with the source value captured in Phase 1. Within the apply phase,
writes proceed in this order:

1. Operand-port writes (`ALU_A`, `LSU_ST_A`, `MUL_A`)
2. All other writes, including FU triggers

This ordering ensures that an operand port and a trigger port of the same FU,
written in the same cycle, work as a unit: the operand latches first, and the
trigger uses the freshly-latched value.

Writes to GPR, BRF, and FU output ports are visible to reads in the *next*
cycle (snapshot-then-apply).

### 7.2 Multiple writes to the same destination

If two or more slots in the same cycle write to the same destination
(including triggering the same FU), behavior is **undefined**. The assembler
must reject such programs.

### 7.3 PC update

If no `GCU_JMP_T` trigger fires in the current cycle, `PC` is incremented by 1
at the end of the cycle.

If exactly one `GCU_JMP_T` fires, `PC` is set to the value moved into the
trigger; the implicit increment does not occur. The new `PC` takes effect on
the next cycle.

If more than one `GCU_JMP_T` fires in a single cycle, behavior is **undefined**.

### 7.4 FU latency

In v1.0, all FU latencies are 1 cycle:

- A trigger written in cycle *N* produces its result at the corresponding
  output port at cycle *N+1*.
- The output port retains the result until a subsequent trigger to the same FU
  overwrites it.
- A read of an output port in cycle *N* returns the value latched by the most
  recent trigger that completed by the start of cycle *N* (i.e., a trigger
  from cycle *N−1* or earlier).

---

## 8. Functional unit reference

### 8.1 ALU

| Component | Spec |
|-----------|------|
| Operand port | `ALU_A` (16 bits, non-triggering) |
| Arithmetic triggers | `ALU_ADD_T`, `ALU_SUB_T`, `ALU_AND_T`, `ALU_OR_T`, `ALU_XOR_T`, `ALU_SHL_T`, `ALU_SHR_T`, `ALU_SSHR_T` |
| Compare triggers | `ALU_EQ_T`, `ALU_NE_T`, `ALU_LT_T`, `ALU_LE_T`, `ALU_GT_T`, `ALU_GE_T` |
| Output ports | `ALU.R` (16-bit arithmetic result), `ALU.P` (1-bit compare result) |
| Latency | 1 |

Compare triggers interpret operands as signed two's-complement 16-bit integers.

Shift triggers use the low 4 bits of the trigger value as the shift count
(0..15). Shift counts ≥ 16 are not architecturally meaningful but produce
the same result as masking to the low 4 bits.

A single ALU trigger per cycle may fire across all four slots; the assembler
must reject any instruction word containing more than one ALU trigger.

### 8.2 LSU

| Component | Spec |
|-----------|------|
| Load trigger | `LSU_LD_T` (value = address; result lands at `LSU.R`) |
| Store address port | `LSU_ST_A` (non-triggering; latches store address) |
| Store data trigger | `LSU_ST_T` (value = data; commits store to `LSU_ST_A`) |
| Output port | `LSU.R` (16 bits) |
| Latency | 1 |

Memory accesses to addresses in the MMIO range (§2.3) are dispatched to
peripheral handlers rather than D-memory.

A single LSU operation per cycle (load or store) may be triggered. The
assembler must reject any instruction word containing more than one LSU
trigger.

### 8.3 GCU

| Component | Spec |
|-----------|------|
| Jump trigger | `GCU_JMP_T` (value = target address) |
| Internal state | `PC` (16 bits, not directly addressable) |
| Latency | 1 (jump takes effect on the cycle following the trigger) |

There is no built-in call, return, or interrupt mechanism in v1.0. Calling
conventions are implemented in software using `LSU` for return-address push
and pop, and `GCU_JMP_T` for the jump.

Reset behavior: `PC ← 0x0000`.

### 8.4 MUL

| Component | Spec |
|-----------|------|
| Operand port | `MUL_A` (16 bits, non-triggering) |
| Trigger | `MUL_T` (value = second operand) |
| Output port | `MUL.R` (16 bits) |
| Latency | 1 |

Computes the low 16 bits of `MUL_A × value`. The low 16 bits are bit-identical
for signed and unsigned multiplication, so a single trigger covers both
interpretations.

---

## 9. Initial state

On reset:

- `PC = 0x0000`
- All GPRs: undefined
- `p0`: undefined (must be initialized before use as a guard)
- All FU output ports: undefined
- D-memory contents: implementation-defined (typically zeroed by the loader)
- I-memory contents: program image loaded by the runtime

A well-formed program initializes any architectural state it depends on
before reading it.

---

## 10. Halt

A program halts by writing any value to MMIO address `0xFFFE` (`HALT`). The
written value becomes the exit code observable to the runtime. After the
store cycle completes, no further instructions are executed.

There is no architectural distinction between halted and running state visible
to the program itself; halt is observed externally.

---

## 11. Undefined behavior

The following situations have **undefined** behavior in v1.0; an emulator
may flag them, halt, or produce arbitrary results. Assemblers and compilers
must not emit code that exhibits any of them.

- Multiple slots in the same cycle writing to the same destination
- Multiple `GCU_JMP_T` triggers firing in the same cycle
- More than one ALU trigger or LSU trigger firing in the same cycle
- Reading an FU output port before any trigger has produced a defined value
- Reading `p0` as a guard before it has been initialized
- Use of any reserved socket ID
- Non-zero bits in the `reserved` field of a slot
- Writes to reserved or read-only MMIO addresses
- Loads or stores to unmapped addresses outside RAM or MMIO ranges
- Use of `IMMEDIATE` as a source with a `src.data` value whose bit-width
  exceeds the destination's expected operand width without an explicit
  truncation rule

---

## 12. Reserved for future revisions

The following are explicitly reserved for post-v1.0 extension and should not
be relied upon by v1.0 programs or implementations:

- `src.sock` codes 22..31 — additional FU output ports
- `dst.sock` codes 39..63 — additional FU input/trigger ports
- `reserved` field of each slot (bits [2:0]) — additional guard polarities,
  long-immediate continuation, or per-slot opcode extensions
- Multi-cycle FU latencies and the associated programming model
- Interrupt and exception delivery
- Unsigned compare triggers, byte-granular memory access, 32-bit MUL high
  product, and a second LSU
