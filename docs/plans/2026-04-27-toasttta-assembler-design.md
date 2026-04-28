# ToastTTA Assembler — Design

**Date:** 2026-04-27
**Status:** Approved, ready for implementation plan
**Crate:** `asm/` (new), workspace member alongside `emu/`

---

## 1. Goals & non-goals

The assembler is the **foundation for a future compiler stack**. It is the lowest layer that consumes text and produces ToastTTA binary images. Its responsibilities end at correct, predictable transcription; *all* optimization, scheduling, and instruction selection live upstream in the compiler.

**Goals**

- Accept hand-written or compiler-emitted `.tasm` source.
- Produce binary files byte-identical to the format the existing emulator already loads.
- Reject programs that exhibit ToastTTA "undefined behavior" per spec §11.
- Report errors with file/line/column and source snippets.
- Reuse the existing `toasttta` library for `Slot`/`IWord` encoding — no duplication.

**Non-goals (v1)**

- No instruction-level shorthand. The user does not write `add r3, r1, r2` and have it lower into 3 cycles. They write the moves.
- No user-defined or built-in macros. No `call`/`ret`/`push`/`pop` shortcuts.
- No automatic scheduling, NOP insertion *across* cycles, or slot reordering.
- No optimization passes.
- No data section, `.org`, or `.byte` directives. Initial D-mem state is zeros; programs initialize their own data via stores.

---

## 2. Architecture

```
              source.tasm
                  │
              ┌───▼───┐
              │ Lexer │  char-by-char state machine
              └───┬───┘
                  │  Vec<Token>
              ┌───▼────┐
              │ Parser │  recursive descent
              └───┬────┘
                  │  Vec<Line>
            ┌─────▼──────┐
            │ Pass 1:    │  - assign I-mem addresses
            │ encode +   │  - record symbols (labels, .equ)
            │ validate   │  - validate per-cycle constraints
            └─────┬──────┘  - resolve backward refs immediately
                  │         - record forward refs as patches
                  │  Vec<IWord> + pending_patches
            ┌─────▼──────┐
            │ Pass 2:    │  resolve forward symbol references
            │ backpatch  │  by re-encoding affected slots
            └─────┬──────┘
                  │
                  ▼
              prog.bin   raw 16-byte little-endian IWords
```

The output is **the same format the emulator already accepts**: each 128-bit IWord written little-endian, no header. No changes required to the emulator.

---

## 3. Source language

### 3.1 Tokens

| Kind | Form |
|---|---|
| identifier | `[A-Za-z_][A-Za-z0-9_]*` |
| number | `42`, `-42`, `0xFF00`, `0b1010` |
| char | `'A'`, `'\n'`, `'\t'`, `'\\'`, `'\''` |
| keyword | `.equ` |
| punct | `#  ->  ;  :  [  ]  !  =` |
| comment | `//` to end of line (skipped) |
| newline | significant — ends a cycle |

**Case sensitivity.** Built-in identifiers (registers, sockets) are case-insensitive: `r0`, `R0`, `alu_r`, `ALU_R` all match the same socket. User-defined symbols (labels and `.equ` names) are case-sensitive.

### 3.2 Move syntax

```
[ guard ]   source   ->   destination
```

Sources:

- Register: `r0` … `r15`
- Predicate: `p0`
- FU output port: `ALU_R`, `ALU_P`, `LSU_R`, `MUL_R`
- Immediate (any 16-bit value): `#42`, `#-1`, `#0xFF01`, `#'A'`, `#label_name`, `#CONSTANT`

Destinations:

- Register: `r0` … `r15`
- Predicate: `p0`
- FU input port: `ALU_A`, `ALU_ADD_T`, `ALU_SUB_T`, `ALU_AND_T`, `ALU_OR_T`,
  `ALU_XOR_T`, `ALU_SHL_T`, `ALU_SHR_T`, `ALU_SSHR_T`,
  `ALU_EQ_T`, `ALU_NE_T`, `ALU_LT_T`, `ALU_LE_T`, `ALU_GT_T`, `ALU_GE_T`,
  `LSU_LD_T`, `LSU_ST_A`, `LSU_ST_T`, `GCU_JMP_T`, `MUL_A`, `MUL_T`, `DISCARD`

Guards (optional prefix on a slot):

- (none) → spec guard `always`
- `[p0]` → spec guard `if p0`
- `[!p0]` → spec guard `if !p0`

There is **no syntax for the `never` guard.** Auto-NOP fills any unwritten slots in a cycle, and that uses the `never` guard internally — users never need to write it.

### 3.3 Cycle structure

A non-empty, non-label, non-directive line constitutes one instruction word (one cycle). Slots within the cycle are separated by `;`. Trailing `;` is allowed. The assembler **auto-NOP-pads** any unwritten slots in the cycle.

```
loop:
  r1 -> ALU_A; #10 -> ALU_LE_T              // 2 slots, 2 auto-NOPs
  ALU_P -> p0                               // 1 slot, 3 auto-NOPs
  [!p0] #end -> GCU_JMP_T; r0 -> ALU_A; r1 -> ALU_ADD_T   // 3 slots, 1 auto-NOP
```

Slot-to-bus assignment is irrelevant (there are no per-bus resource differences). The assembler packs source-order moves into slots 0..N and NOP-fills slots N..3.

### 3.4 Labels

```
label_name:
```

Resolves to the I-mem address of the *next* instruction word. A label may sit on its own line (as above) or precede a cycle on the same line:

```
loop: r1 -> ALU_A; #10 -> ALU_LE_T
```

Labels and `.equ` symbols share one namespace; collisions are an error. Labels can be referenced as immediates anywhere a number can appear, prefixed with `#`: `#loop`, `#end`.

### 3.5 `.equ` constants

```
.equ NAME VALUE
```

Defines a 16-bit symbolic constant. `VALUE` is a numeric or character literal — *not* an expression. The constant is then usable as `#NAME` in any immediate position.

```
.equ STDOUT_INT 0xFF01
.equ HALT       0xFFFE
.equ MAX_ITER   16
```

Forward references to `.equ` symbols are resolved in pass 2 like labels.

### 3.6 Comments

`//` to end of line. No block comments in v1.

---

## 4. Validation

The following are rejected at assemble time. They mirror the spec's "undefined behavior" list (§11) plus a few syntactic guards.

| # | Condition |
|---|---|
| V1 | Multiple slots in one cycle write the same destination |
| V2 | More than one ALU trigger in one cycle (any of `ALU_*_T`) |
| V3 | More than one LSU trigger in one cycle (any of `LSU_LD_T`, `LSU_ST_T`) |
| V4 | More than one MUL trigger in one cycle (`MUL_T`) |
| V5 | More than one `GCU_JMP_T` in one cycle |
| V6 | More than four slots in one cycle |
| V7 | Reserved or unknown socket name (e.g., `MUL_HI_T` doesn't exist in v1.0) |
| V8 | Immediate value outside `−32768 … 65535` |
| V9 | Reference to an undefined label or `.equ` symbol after pass 2 |
| V10 | Two `.equ` or label definitions with the same name |

`r0 -> ALU_A` *alone* (operand-port write without trigger) is **not** rejected. It is a valid, useful operation: latch the ALU operand register for the next cycle's trigger. Many real schedules rely on this.

Errors are collected, not fatal-on-first. Both passes record diagnostics; the assembler exits non-zero only if any errors accumulated.

---

## 5. Implementation

### 5.1 Crate layout

Promote the repository to a Cargo workspace:

```
ToastTTA/
├── Cargo.toml         ← workspace root
├── emu/               ← existing
├── asm/               ← new
│   ├── Cargo.toml         depends on `toasttta` (emu's lib)
│   ├── src/
│   │   ├── lib.rs         pub fn assemble(&str) -> Result<Vec<IWord>, Diagnostics>
│   │   ├── main.rs        CLI binary
│   │   ├── lexer.rs
│   │   ├── parser.rs
│   │   ├── encoder.rs
│   │   └── diag.rs        diagnostic types and rendering
│   └── tests/
│       └── integration.rs
└── docs/plans/
```

`asm/Cargo.toml` declares a `path = "../emu"` dependency on the `toasttta` library and re-uses `Slot`, `IWord`, `isa::src`, `isa::dst`, `isa::guard` directly. No code duplication.

### 5.2 Lexer

Hand-written, ~100 lines. Single forward pass over chars with explicit state. Produces `Vec<Token>` where `Token` carries kind + span.

The lexer does **not** distinguish socket names from user identifiers; it emits `Identifier` for both. The parser does the lookup against a static `HashMap<&'static str, SocketKind>` table.

### 5.3 Parser

Recursive-descent, line-oriented. Grammar:

```ebnf
program     = { line } EOF .
line        = label-decl | equ-decl | cycle | empty .
label-decl  = identifier ":" [ cycle ] newline .
equ-decl    = ".equ" identifier literal newline .
cycle       = slot { ";" slot } [ ";" ] newline .
slot        = [ guard ] source "->" destination .
guard       = "[" [ "!" ] "p0" "]" .
source      = register | predicate | fu-output | "#" immediate .
destination = register | predicate | fu-input .
immediate   = number | char | identifier .
empty       = newline .
```

Each line is parsed independently; recovery on syntax errors resumes at the next newline.

The parser produces:

```rust
pub enum Line {
    Label { name: String, span: Span, attached: Option<CycleSpec> },
    Equ   { name: String, value: u16, span: Span },
    Cycle (CycleSpec),
    Empty,
}

pub struct CycleSpec {
    pub slots: Vec<SlotSpec>,    // 1..=4
    pub span: Span,
}

pub struct SlotSpec {
    pub guard: Guard,
    pub src: Source,
    pub dst: Destination,
    pub span: Span,
}

pub enum Source {
    Gpr(u8),
    Brf,
    FuOut(SocketId),
    Imm(ImmExpr),
}

pub enum ImmExpr {
    Literal(u16),
    Symbol(String),
}

pub enum Destination {
    Gpr(u8),
    Brf,
    FuIn(SocketId),
    Discard,
}
```

### 5.4 Pass 1: encode + validate + record symbols

Walks `Vec<Line>` once with a running `addr: u16`:

- `Label` → record `addr` in `symbols: HashMap<String, u16>`. Collision → V10 error.
  If the label has an attached cycle, fall through to encoding it.
- `Equ` → record value in `symbols`. Collision → V10 error.
- `Cycle` → run validation (V1–V8). On error, skip encoding and continue. On success:
  encode each slot via `Slot::new(...).encode()`. Backward symbol references resolve
  immediately via `symbols.get`. Forward references store `0` in the immediate field
  and add `(addr, slot_idx, name, span)` to `pending_patches`.
- After a cycle: `addr = addr.wrapping_add(1)`.

Validation is implemented as a per-cycle linter that examines all slots together:

```rust
fn validate(slots: &[SlotSpec]) -> Vec<Diag> {
    let mut diags = Vec::new();
    if slots.len() > 4 { diags.push(...V6...); }
    // V1: count writes per dst
    // V2-V5: count triggers per FU
    // V7: check socket name validity
    // V8: check immediate range
    diags
}
```

### 5.5 Pass 2: backpatch forward references

```rust
for (addr, slot_idx, name, span) in pending_patches {
    match symbols.get(&name) {
        Some(&value) => iwords[addr as usize].slots[slot_idx]
            .src_data = value,
        None => diags.push(Diag::undefined_symbol(name, span)),  // V9
    }
}
```

After pass 2, if any diags accumulated across both passes, exit with non-zero status.

### 5.6 Diagnostic format

```rust
pub struct Diag {
    pub severity: Severity,    // Error | Warning
    pub code: DiagCode,        // V1..V10, plus syntactic codes
    pub message: String,
    pub span: Span,
}

pub struct Span {
    pub file: Arc<str>,
    pub line: u32,             // 1-indexed
    pub col: u32,              // 1-indexed
    pub len: u32,              // bytes
}
```

Rendered as:

```
error[V2]: more than one ALU trigger in a single cycle
  --> mandelbrot.tasm:42:24
   |
42 |   r1 -> ALU_ADD_T; r2 -> ALU_SUB_T
   |        ^^^^^^^^^       ^^^^^^^^^
   |
   = note: ToastTTA v1.0 has a single ALU FU; only one ALU trigger may fire per
     instruction word (spec §8.1).
```

---

## 6. Testing

Three layers, each with deliberate coverage:

1. **Lexer unit tests** — small inputs, expected token streams. Every token kind, every escape sequence, every error case.
2. **Parser unit tests** — small inputs, expected `Vec<Line>` shapes. Every grammar production.
3. **End-to-end golden tests** — re-write the three existing examples (`sample_prog`, `fib`, `mandelbrot`) as `.tasm` files. Assemble them, byte-compare against the hand-coded `Rust → IWord::encode → bytes` outputs the emulator's `examples/*.rs` already produces.

The golden test for `mandelbrot.tasm` is the strongest single regression we could write: 46 instruction words, 736 bytes, three nested loops, predicated branches, fixed-point math, MMIO output. If the assembled bytes match the hand-coded bytes, the entire stack works.

---

## 7. CLI

```
toasttta-asm INPUT.tasm [-o OUTPUT.bin]      # default OUTPUT = stem(INPUT) + ".bin"
toasttta-asm INPUT.tasm --dump-tokens        # print token stream, exit
toasttta-asm INPUT.tasm --dump-ast           # print parsed Lines, exit
```

Exit codes: `0` success, `1` assembly errors reported, `2` IO/usage error.

---

## 8. Future work (out of scope for v1)

- Instruction-level shorthand (`add r3, r1, r2`) when/if the compiler wants to emit at that level.
- User-defined macros via `.macro` / `.endm`.
- A `.org` directive for absolute placement.
- A `.data` section + `.byte`/`.word` for D-mem initialization image.
- Constant arithmetic in `.equ` (`.equ DOUBLE = 2 * BASE`).
- A `--watch` flag that re-assembles on file change.
- Better diagnostics: did-you-mean for typo'd socket names, related-spans for V1/V2/V3/V4/V5.

---

## Appendix A — example .tasm

A hand-written translation of `sample_prog.rs`:

```
// 3 + 4 = 7, print, halt with exit code 7.

.equ STDOUT_INT  0xFF01
.equ HALT        0xFFFE

main:
  #3 -> ALU_A; #4 -> ALU_ADD_T
  ALU_R -> r0
  #STDOUT_INT -> LSU_ST_A; r0 -> LSU_ST_T
  #'\n' -> LSU_ST_A                          // wrong on purpose — see note
  #HALT -> LSU_ST_A; r0 -> LSU_ST_T
```

(The 4th cycle is intentionally wrong to demonstrate that V1–V8 catch real
mistakes: `#'\n' -> LSU_ST_A` writes the wrong address into the LSU operand
port without firing a store. The assembler doesn't reject this — it's
syntactically valid and might be intentional — but the program prints garbage.
Distinguishing "syntactically valid but probably wrong" from "actually invalid"
is exactly the line v1 doesn't try to police.)
