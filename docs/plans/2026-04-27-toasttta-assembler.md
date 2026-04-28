# ToastTTA Assembler Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a minimal text-to-binary assembler for ToastTTA that consumes `.tasm` source and produces the same `.bin` format the existing emulator accepts.

**Architecture:** Hand-written lexer + recursive-descent parser feeding a two-pass encoder. New `asm/` crate in a Cargo workspace, depending on `emu/`'s `toasttta` library for `Slot`/`IWord` encoding. No external dependencies beyond stdlib.

**Tech Stack:** Rust 2021, stdlib only, `#[cfg(test)]` unit tests, integration tests in `asm/tests/`.

**Reference design doc:** `docs/plans/2026-04-27-toasttta-assembler-design.md`

---

## Conventions for this plan

- All test code lives in `#[cfg(test)] mod tests` blocks at the bottom of each source file, **except** the three golden tests in Phase 8 (which live in `asm/tests/`).
- Cargo commands run from the repo root (`/Users/howard/src/ToastTTA`).
- Commit messages use conventional commits: `feat(asm)`, `test(asm)`, `chore`, `refactor`.
- Every task ends with a commit. Frequent small commits are intentional.
- "Run: …" steps state the **exact** command and the expected pass/fail behavior.

---

## Phase 1 — Workspace setup (2 tasks)

### Task 1: Promote repo to a Cargo workspace

**Files:**
- Create: `Cargo.toml` (repo root)

**Step 1: Create the workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "emu",
    "asm",
]
```

**Step 2: Verify the existing emu still builds**

Run: `cargo build -p toasttta-emu`
Expected: PASS — should compile cleanly. (`asm` doesn't exist yet but the workspace member entry is fine; cargo just warns until we create it.)

If cargo errors on the missing `asm` member, temporarily remove it from `members`, then add it back in Task 2.

**Step 3: Run the emu tests to ensure no regression**

Run: `cargo test -p toasttta-emu`
Expected: PASS — the 3 unit tests + 3 integration tests we already have.

**Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "chore: promote repo to a Cargo workspace"
```

---

### Task 2: Scaffold the asm crate

**Files:**
- Create: `asm/Cargo.toml`
- Create: `asm/src/lib.rs`
- Create: `asm/src/main.rs`

**Step 1: Create asm/Cargo.toml**

```toml
[package]
name = "toasttta-asm"
version = "0.1.0"
edition = "2021"
description = "Assembler for the ToastTTA transport-triggered architecture"

[lib]
name = "toasttta_asm"
path = "src/lib.rs"

[[bin]]
name = "toasttta-asm"
path = "src/main.rs"

[dependencies]
toasttta = { path = "../emu" }
```

**Step 2: Create asm/src/lib.rs (stub)**

```rust
//! ToastTTA assembler library.
//!
//! See `docs/plans/2026-04-27-toasttta-assembler-design.md`.

use toasttta::IWord;

/// Top-level assembly entry point. Lex → parse → encode.
///
/// Returns the assembled instruction words on success, or a non-empty list
/// of diagnostics on failure.
pub fn assemble(_source: &str, _filename: &str) -> Result<Vec<IWord>, Vec<String>> {
    todo!("implemented incrementally — see plan")
}
```

**Step 3: Create asm/src/main.rs (stub)**

```rust
fn main() {
    eprintln!("toasttta-asm: not yet implemented");
    std::process::exit(2);
}
```

**Step 4: Verify the workspace builds**

Run: `cargo build --workspace`
Expected: PASS — both crates compile (with a `dead_code` warning on the stub return type, which is fine).

**Step 5: Commit**

```bash
git add asm/Cargo.toml asm/src/lib.rs asm/src/main.rs
git commit -m "feat(asm): scaffold toasttta-asm crate"
```

---

## Phase 2 — Diagnostics (1 task)

### Task 3: Diagnostic types

**Files:**
- Create: `asm/src/diag.rs`
- Modify: `asm/src/lib.rs` (add `mod diag;`)

**Step 1: Write the failing test**

Add to `asm/src/diag.rs`:

```rust
//! Diagnostic types for the assembler.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub file: std::sync::Arc<str>,
    pub line: u32,   // 1-indexed
    pub col:  u32,   // 1-indexed
    pub len:  u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diag {
    pub severity: Severity,
    pub message:  String,
    pub span:     Span,
}

#[derive(Default, Clone, Debug)]
pub struct Diagnostics {
    pub items: Vec<Diag>,
}

impl Diagnostics {
    pub fn new() -> Self { Self::default() }

    pub fn push(&mut self, d: Diag) { self.items.push(d); }

    pub fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.push(Diag { severity: Severity::Error, message: msg.into(), span });
    }

    pub fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.push(Diag { severity: Severity::Warning, message: msg.into(), span });
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn span() -> Span {
        Span { file: Arc::from("test.tasm"), line: 1, col: 1, len: 1 }
    }

    #[test]
    fn collect_and_query() {
        let mut d = Diagnostics::new();
        assert!(!d.has_errors());
        d.warn(span(), "watch out");
        assert!(!d.has_errors());
        d.error(span(), "oh no");
        assert!(d.has_errors());
        assert_eq!(d.items.len(), 2);
    }
}
```

Add to `asm/src/lib.rs`:

```rust
pub mod diag;
```

**Step 2: Run the test to verify it fails**

Run: `cargo test -p toasttta-asm collect_and_query`
Expected: PASS — actually this test should PASS immediately because the implementation is included. The "failing test" pattern here is degenerate: we're introducing types whose structural correctness is the test. If it doesn't compile, that's the failure.

**Step 3: Verify the test passes**

Run: `cargo test -p toasttta-asm collect_and_query`
Expected: PASS.

**Step 4: Commit**

```bash
git add asm/src/diag.rs asm/src/lib.rs
git commit -m "feat(asm): add diagnostic types (Span, Diag, Diagnostics)"
```

---

## Phase 3 — Lexer (8 tasks)

### Task 4: Token types

**Files:**
- Create: `asm/src/lexer.rs`
- Modify: `asm/src/lib.rs` (add `mod lexer;`)

**Step 1: Write the test + impl**

`asm/src/lexer.rs`:

```rust
//! Hand-written ToastTTA assembler lexer.

use crate::diag::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    Number(i64),       // signed wide enough to hold any literal we'll emit
    Char(u16),         // already-resolved code point (low byte = ASCII)
    KwEqu,             // .equ
    Hash,              // #
    Arrow,             // ->
    Semi,              // ;
    Colon,             // :
    LBracket,          // [
    RBracket,          // ]
    Bang,              // !
    Eq,                // =
    Newline,           // significant
    Eof,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_kind_equality() {
        assert_eq!(TokenKind::Hash, TokenKind::Hash);
        assert_ne!(TokenKind::Hash, TokenKind::Bang);
    }
}
```

Add to `asm/src/lib.rs`:

```rust
pub mod lexer;
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm token_kind_equality`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/lexer.rs asm/src/lib.rs
git commit -m "feat(asm): define lexer Token and TokenKind"
```

---

### Task 5: Lexer skeleton — whitespace and newlines

**Files:**
- Modify: `asm/src/lexer.rs`

**Step 1: Write the failing test**

Add to `asm/src/lexer.rs`:

```rust
pub fn lex(source: &str, filename: &str) -> Result<Vec<Token>, crate::diag::Diagnostics> {
    let mut lexer = Lexer::new(source, filename);
    lexer.run();
    if lexer.diags.has_errors() {
        Err(lexer.diags)
    } else {
        Ok(lexer.tokens)
    }
}

struct Lexer<'src> {
    src: &'src [u8],
    pos: usize,
    line: u32,
    line_start: usize,
    file: std::sync::Arc<str>,
    tokens: Vec<Token>,
    diags: crate::diag::Diagnostics,
}

impl<'src> Lexer<'src> {
    fn new(source: &'src str, filename: &str) -> Self {
        Self {
            src: source.as_bytes(),
            pos: 0,
            line: 1,
            line_start: 0,
            file: std::sync::Arc::from(filename),
            tokens: Vec::new(),
            diags: crate::diag::Diagnostics::new(),
        }
    }

    fn col(&self) -> u32 {
        (self.pos - self.line_start + 1) as u32
    }

    fn span(&self, start: usize, len: u32) -> Span {
        Span {
            file: self.file.clone(),
            line: self.line,
            col: (start - self.line_start + 1) as u32,
            len,
        }
    }

    fn peek(&self) -> Option<u8> { self.src.get(self.pos).copied() }
    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.line_start = self.pos;
        }
        Some(c)
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\r' => { self.bump(); }
                b'\n' => {
                    let span = self.span(self.pos, 1);
                    self.bump();
                    self.tokens.push(Token { kind: TokenKind::Newline, span });
                }
                _ => {
                    let span = self.span(self.pos, 1);
                    self.diags.error(span, format!("unexpected character {:?}", c as char));
                    self.bump(); // skip and continue
                }
            }
        }
        let span = self.span(self.pos, 0);
        self.tokens.push(Token { kind: TokenKind::Eof, span });
    }
}

#[cfg(test)]
mod whitespace_tests {
    use super::*;

    #[test]
    fn skip_whitespace_emit_newlines() {
        let toks = lex("  \n  \n", "x.tasm").unwrap();
        // Two Newlines + Eof.
        assert_eq!(toks.len(), 3);
        assert!(matches!(toks[0].kind, TokenKind::Newline));
        assert!(matches!(toks[1].kind, TokenKind::Newline));
        assert!(matches!(toks[2].kind, TokenKind::Eof));
    }

    #[test]
    fn unknown_char_diagnoses_but_continues() {
        let result = lex("@\n", "x.tasm");
        assert!(result.is_err());
    }
}
```

**Step 2: Run the test**

Run: `cargo test -p toasttta-asm whitespace`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/lexer.rs
git commit -m "feat(asm): lexer skeleton with whitespace and newline handling"
```

---

### Task 6: Lex `//` comments

**Files:**
- Modify: `asm/src/lexer.rs`

**Step 1: Add the failing test, then fix**

In the `match c` block of `Lexer::run`, before the `_` arm, add:

```rust
b'/' if self.src.get(self.pos + 1) == Some(&b'/') => {
    while let Some(c) = self.peek() {
        if c == b'\n' { break; }
        self.bump();
    }
}
```

Append to the existing `whitespace_tests` mod:

```rust
#[test]
fn line_comments_skipped() {
    let toks = lex("// hello world\n  // another\n", "x.tasm").unwrap();
    assert_eq!(toks.len(), 3); // two newlines + EOF
    assert!(matches!(toks[0].kind, TokenKind::Newline));
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm comments`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/lexer.rs
git commit -m "feat(asm): lex // line comments"
```

---

### Task 7: Lex identifiers and `.equ` keyword

**Files:**
- Modify: `asm/src/lexer.rs`

**Step 1: Implement and test**

Add this helper to the `Lexer` impl:

```rust
fn lex_ident(&mut self) -> (String, Span) {
    let start = self.pos;
    while let Some(c) = self.peek() {
        if c.is_ascii_alphanumeric() || c == b'_' { self.bump(); } else { break; }
    }
    let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap().to_string();
    let span = self.span(start, (self.pos - start) as u32);
    (s, span)
}
```

Add new arms in `run` *before* the `_` (unknown) arm:

```rust
b'.' => {
    // Only legal start of `.equ`; anything else is an error.
    let start = self.pos;
    self.bump(); // consume '.'
    let (kw, _) = self.lex_ident();
    let span = self.span(start, (self.pos - start) as u32);
    if kw == "equ" {
        self.tokens.push(Token { kind: TokenKind::KwEqu, span });
    } else {
        self.diags.error(span, format!("unknown directive .{kw}"));
    }
}
c if c.is_ascii_alphabetic() || c == b'_' => {
    let (s, span) = self.lex_ident();
    self.tokens.push(Token { kind: TokenKind::Ident(s), span });
}
```

Add tests:

```rust
#[test]
fn idents_and_equ() {
    let toks = lex("r0 ALU_R foo_bar .equ FOO\n", "x.tasm").unwrap();
    let kinds: Vec<_> = toks.iter().map(|t| &t.kind).collect();
    assert!(matches!(kinds[0], TokenKind::Ident(s) if s == "r0"));
    assert!(matches!(kinds[1], TokenKind::Ident(s) if s == "ALU_R"));
    assert!(matches!(kinds[2], TokenKind::Ident(s) if s == "foo_bar"));
    assert!(matches!(kinds[3], TokenKind::KwEqu));
    assert!(matches!(kinds[4], TokenKind::Ident(s) if s == "FOO"));
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm idents_and_equ`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/lexer.rs
git commit -m "feat(asm): lex identifiers and .equ keyword"
```

---

### Task 8: Lex numbers (decimal, hex, binary, signed)

**Files:**
- Modify: `asm/src/lexer.rs`

**Step 1: Implement and test**

Add this helper to `Lexer`:

```rust
fn lex_number(&mut self, signed_negative: bool) -> Token {
    let start = if signed_negative { self.pos - 1 } else { self.pos };

    let mut radix: u32 = 10;
    if self.peek() == Some(b'0') {
        match self.src.get(self.pos + 1) {
            Some(b'x') | Some(b'X') => { self.bump(); self.bump(); radix = 16; }
            Some(b'b') | Some(b'B') => { self.bump(); self.bump(); radix = 2; }
            _ => {}
        }
    }

    let digits_start = self.pos;
    while let Some(c) = self.peek() {
        let ok = match radix {
            10 => c.is_ascii_digit(),
            16 => c.is_ascii_hexdigit(),
            2  => c == b'0' || c == b'1',
            _  => false,
        };
        if !ok { break; }
        self.bump();
    }

    let body = std::str::from_utf8(&self.src[digits_start..self.pos]).unwrap();
    let value = i64::from_str_radix(body, radix).unwrap_or(0);
    let value = if signed_negative { -value } else { value };
    let span = self.span(start, (self.pos - start) as u32);
    Token { kind: TokenKind::Number(value), span }
}
```

Add new arms in `run`:

```rust
b'0'..=b'9' => {
    let tok = self.lex_number(false);
    self.tokens.push(tok);
}
b'-' => {
    self.bump();
    let tok = self.lex_number(true);
    self.tokens.push(tok);
}
```

Add tests:

```rust
#[test]
fn numbers_all_radices() {
    let toks = lex("42 -42 0xFF 0b1010\n", "x.tasm").unwrap();
    let nums: Vec<i64> = toks.iter()
        .filter_map(|t| if let TokenKind::Number(n) = t.kind { Some(n) } else { None })
        .collect();
    assert_eq!(nums, vec![42, -42, 0xFF, 0b1010]);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm numbers_all_radices`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/lexer.rs
git commit -m "feat(asm): lex decimal/hex/binary/negative numbers"
```

---

### Task 9: Lex character literals with escape sequences

**Files:**
- Modify: `asm/src/lexer.rs`

**Step 1: Implement and test**

Add a method:

```rust
fn lex_char(&mut self) {
    let start = self.pos;
    self.bump(); // opening '
    let val = match self.peek() {
        Some(b'\\') => {
            self.bump();
            match self.bump() {
                Some(b'n')  => b'\n',
                Some(b't')  => b'\t',
                Some(b'r')  => b'\r',
                Some(b'\\') => b'\\',
                Some(b'\'') => b'\'',
                Some(b'0')  => 0,
                other => {
                    let span = self.span(start, (self.pos - start) as u32);
                    self.diags.error(span, format!("unknown escape \\{:?}", other));
                    0
                }
            }
        }
        Some(c) => { self.bump(); c }
        None => {
            let span = self.span(start, 1);
            self.diags.error(span, "unterminated char literal");
            0
        }
    };
    if self.peek() == Some(b'\'') {
        self.bump();
    } else {
        let span = self.span(self.pos, 1);
        self.diags.error(span, "expected closing ' for char literal");
    }
    let span = self.span(start, (self.pos - start) as u32);
    self.tokens.push(Token { kind: TokenKind::Char(val as u16), span });
}
```

Add to the `match` in `run`:

```rust
b'\'' => self.lex_char(),
```

Add tests:

```rust
#[test]
fn char_literals() {
    let toks = lex("'A' '\\n' '\\t' '\\\\'\n", "x.tasm").unwrap();
    let chars: Vec<u16> = toks.iter()
        .filter_map(|t| if let TokenKind::Char(v) = t.kind { Some(v) } else { None })
        .collect();
    assert_eq!(chars, vec![b'A' as u16, b'\n' as u16, b'\t' as u16, b'\\' as u16]);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm char_literals`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/lexer.rs
git commit -m "feat(asm): lex character literals with escapes"
```

---

### Task 10: Lex punctuation (including `->`)

**Files:**
- Modify: `asm/src/lexer.rs`

**Step 1: Implement and test**

In the `run` match, add **before** the `b'-'` arm (since `->` shares the prefix with negative numbers, we need to disambiguate):

```rust
b'-' if self.src.get(self.pos + 1) == Some(&b'>') => {
    let span = self.span(self.pos, 2);
    self.bump(); self.bump();
    self.tokens.push(Token { kind: TokenKind::Arrow, span });
}
```

Add the rest of the punct arms:

```rust
b'#' => { let span = self.span(self.pos, 1); self.bump(); self.tokens.push(Token { kind: TokenKind::Hash, span }); }
b';' => { let span = self.span(self.pos, 1); self.bump(); self.tokens.push(Token { kind: TokenKind::Semi, span }); }
b':' => { let span = self.span(self.pos, 1); self.bump(); self.tokens.push(Token { kind: TokenKind::Colon, span }); }
b'[' => { let span = self.span(self.pos, 1); self.bump(); self.tokens.push(Token { kind: TokenKind::LBracket, span }); }
b']' => { let span = self.span(self.pos, 1); self.bump(); self.tokens.push(Token { kind: TokenKind::RBracket, span }); }
b'!' => { let span = self.span(self.pos, 1); self.bump(); self.tokens.push(Token { kind: TokenKind::Bang, span }); }
b'=' => { let span = self.span(self.pos, 1); self.bump(); self.tokens.push(Token { kind: TokenKind::Eq, span }); }
```

Add tests:

```rust
#[test]
fn punctuation() {
    let toks = lex("# -> ; : [ ] ! =\n", "x.tasm").unwrap();
    let kinds: Vec<TokenKind> = toks.iter().map(|t| t.kind.clone()).collect();
    use TokenKind::*;
    assert_eq!(&kinds[..8],
        &[Hash, Arrow, Semi, Colon, LBracket, RBracket, Bang, Eq]);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm punctuation`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/lexer.rs
git commit -m "feat(asm): lex punctuation tokens including ->"
```

---

### Task 11: Lexer end-to-end integration test

**Files:**
- Modify: `asm/src/lexer.rs`

**Step 1: Add a comprehensive test**

```rust
#[test]
fn full_cycle_lexes_correctly() {
    let src = "// init\n.equ STDOUT 0xFF01\nmain: r0 -> ALU_A; #4 -> ALU_ADD_T\n";
    let toks = lex(src, "x.tasm").unwrap();
    let kinds: Vec<TokenKind> = toks.iter().map(|t| t.kind.clone()).collect();
    use TokenKind::*;
    assert_eq!(kinds, vec![
        Newline,                                                  // after // comment
        KwEqu, Ident("STDOUT".into()), Number(0xFF01), Newline,
        Ident("main".into()), Colon,
        Ident("r0".into()), Arrow, Ident("ALU_A".into()),
        Semi,
        Hash, Number(4), Arrow, Ident("ALU_ADD_T".into()),
        Newline,
        Eof,
    ]);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm full_cycle_lexes_correctly`
Expected: PASS — this exercises every token kind together.

**Step 3: Commit**

```bash
git add asm/src/lexer.rs
git commit -m "test(asm): end-to-end lexer integration test"
```

---

## Phase 4 — Parser (10 tasks)

### Task 12: AST types

**Files:**
- Create: `asm/src/parser.rs`
- Modify: `asm/src/lib.rs` (add `mod parser;`)

**Step 1: Create the AST types**

`asm/src/parser.rs`:

```rust
//! Recursive-descent parser for ToastTTA assembly.

use crate::diag::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Empty,
    Label  { name: String, span: Span, attached: Option<CycleSpec> },
    Equ    { name: String, value: u16, span: Span },
    Cycle  (CycleSpec),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CycleSpec {
    pub slots: Vec<SlotSpec>,
    pub span:  Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotSpec {
    pub guard: Guard,
    pub src:   Source,
    pub dst:   Destination,
    pub span:  Span,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Guard {
    Always,
    IfP0,
    IfNotP0,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Gpr(u8),                  // 0..15
    BrfP0,
    AluR, AluP, LsuR, MulR,
    Imm(ImmExpr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImmExpr {
    Literal(u16),
    Symbol(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Gpr(u8),
    BrfP0,
    AluA,
    AluAddT, AluSubT, AluAndT, AluOrT, AluXorT,
    AluShlT, AluShrT, AluSshrT,
    AluEqT, AluNeT, AluLtT, AluLeT, AluGtT, AluGeT,
    LsuLdT, LsuStA, LsuStT,
    GcuJmpT,
    Discard,
    MulA, MulT,
}

#[cfg(test)]
mod ast_smoke_tests {
    use super::*;
    #[test]
    fn variants_distinct() {
        assert_ne!(Source::AluR, Source::AluP);
        assert_ne!(Destination::AluA, Destination::AluAddT);
    }
}
```

Add to `asm/src/lib.rs`:

```rust
pub mod parser;
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm variants_distinct`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/parser.rs asm/src/lib.rs
git commit -m "feat(asm): define parser AST types"
```

---

### Task 13: Socket name lookup tables (case-insensitive)

**Files:**
- Modify: `asm/src/parser.rs`

**Step 1: Add lookup tables and tests**

```rust
/// Look up an identifier as a source. Case-insensitive.
pub fn source_from_name(name: &str) -> Option<Source> {
    let lower = name.to_ascii_lowercase();
    if let Some(stripped) = lower.strip_prefix('r') {
        if let Ok(n) = stripped.parse::<u8>() {
            if n < 16 { return Some(Source::Gpr(n)); }
        }
    }
    Some(match lower.as_str() {
        "p0"     => Source::BrfP0,
        "alu_r"  => Source::AluR,
        "alu_p"  => Source::AluP,
        "lsu_r"  => Source::LsuR,
        "mul_r"  => Source::MulR,
        _        => return None,
    })
}

/// Look up an identifier as a destination. Case-insensitive.
pub fn destination_from_name(name: &str) -> Option<Destination> {
    let lower = name.to_ascii_lowercase();
    if let Some(stripped) = lower.strip_prefix('r') {
        if let Ok(n) = stripped.parse::<u8>() {
            if n < 16 { return Some(Destination::Gpr(n)); }
        }
    }
    Some(match lower.as_str() {
        "p0"        => Destination::BrfP0,
        "alu_a"     => Destination::AluA,
        "alu_add_t" => Destination::AluAddT,
        "alu_sub_t" => Destination::AluSubT,
        "alu_and_t" => Destination::AluAndT,
        "alu_or_t"  => Destination::AluOrT,
        "alu_xor_t" => Destination::AluXorT,
        "alu_shl_t" => Destination::AluShlT,
        "alu_shr_t" => Destination::AluShrT,
        "alu_sshr_t"=> Destination::AluSshrT,
        "alu_eq_t"  => Destination::AluEqT,
        "alu_ne_t"  => Destination::AluNeT,
        "alu_lt_t"  => Destination::AluLtT,
        "alu_le_t"  => Destination::AluLeT,
        "alu_gt_t"  => Destination::AluGtT,
        "alu_ge_t"  => Destination::AluGeT,
        "lsu_ld_t"  => Destination::LsuLdT,
        "lsu_st_a"  => Destination::LsuStA,
        "lsu_st_t"  => Destination::LsuStT,
        "gcu_jmp_t" => Destination::GcuJmpT,
        "discard"   => Destination::Discard,
        "mul_a"     => Destination::MulA,
        "mul_t"     => Destination::MulT,
        _ => return None,
    })
}

#[cfg(test)]
mod lookup_tests {
    use super::*;

    #[test]
    fn gpr_source() {
        assert_eq!(source_from_name("r0"),  Some(Source::Gpr(0)));
        assert_eq!(source_from_name("R15"), Some(Source::Gpr(15)));
        assert_eq!(source_from_name("r16"), None);
    }

    #[test]
    fn fu_sources_case_insensitive() {
        assert_eq!(source_from_name("ALU_R"), Some(Source::AluR));
        assert_eq!(source_from_name("alu_r"), Some(Source::AluR));
        assert_eq!(source_from_name("Mul_R"), Some(Source::MulR));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(source_from_name("garbage"),       None);
        assert_eq!(destination_from_name("MUL_HI_T"), None);
    }

    #[test]
    fn destinations_complete() {
        assert_eq!(destination_from_name("ALU_ADD_T"), Some(Destination::AluAddT));
        assert_eq!(destination_from_name("GCU_JMP_T"), Some(Destination::GcuJmpT));
        assert_eq!(destination_from_name("DISCARD"),   Some(Destination::Discard));
    }
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm lookup`
Expected: PASS — all four tests.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "feat(asm): socket name lookup tables (case-insensitive)"
```

---

### Task 14: Parser skeleton + empty/newline handling

**Files:**
- Modify: `asm/src/parser.rs`

**Step 1: Add the parser shell**

```rust
use crate::diag::{Diagnostics, Span};
use crate::lexer::{Token, TokenKind};

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Line>, Diagnostics> {
    let mut p = Parser::new(tokens);
    p.parse_program();
    if p.diags.has_errors() { Err(p.diags) } else { Ok(p.lines) }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    lines: Vec<Line>,
    diags: Diagnostics,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, lines: Vec::new(), diags: Diagnostics::new() }
    }

    fn peek(&self) -> &TokenKind { &self.tokens[self.pos].kind }
    fn peek_span(&self) -> Span { self.tokens[self.pos].span.clone() }
    fn bump(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() { self.pos += 1; }
        t
    }
    fn eat(&mut self, kind: &TokenKind) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(kind) {
            self.bump();
            true
        } else { false }
    }

    fn parse_program(&mut self) {
        while !matches!(self.peek(), TokenKind::Eof) {
            if matches!(self.peek(), TokenKind::Newline) {
                self.bump();
                self.lines.push(Line::Empty);
                continue;
            }
            // Other line kinds wired up in subsequent tasks.
            // For now, skip unknown tokens by advancing.
            self.bump();
        }
    }
}

#[cfg(test)]
mod parse_skeleton_tests {
    use super::*;
    use crate::lexer::lex;

    #[test]
    fn empty_input() {
        let toks = lex("", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn empty_lines() {
        let toks = lex("\n\n\n", "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        assert_eq!(lines, vec![Line::Empty, Line::Empty, Line::Empty]);
    }
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm parse_skeleton`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "feat(asm): parser skeleton with empty-line handling"
```

---

### Task 15: Parse `.equ` declarations

**Files:**
- Modify: `asm/src/parser.rs`

**Step 1: Implement**

Replace the `// Other line kinds...` placeholder in `parse_program` with:

```rust
match self.peek().clone() {
    TokenKind::KwEqu => self.parse_equ(),
    TokenKind::Newline => { self.bump(); self.lines.push(Line::Empty); }
    _ => { self.bump(); /* will be handled by later tasks */ }
}
```

Add:

```rust
impl Parser {
    fn parse_equ(&mut self) {
        let kw_span = self.peek_span();
        self.bump(); // .equ

        let name = if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump(); s
        } else {
            self.diags.error(self.peek_span(), "expected identifier after .equ");
            self.skip_to_newline();
            return;
        };

        let value = match self.peek().clone() {
            TokenKind::Number(n) => { self.bump(); n as i64 }
            TokenKind::Char(c)   => { self.bump(); c as i64 }
            _ => {
                self.diags.error(self.peek_span(), "expected literal value after .equ name");
                self.skip_to_newline();
                return;
            }
        };

        if !(value >= -32768 && value <= 65535) {
            self.diags.error(self.peek_span(), format!(".equ value {value} out of 16-bit range"));
        }

        self.expect_newline_or_eof();
        self.lines.push(Line::Equ {
            name,
            value: (value as i32 & 0xFFFF) as u16,
            span: kw_span,
        });
    }

    fn skip_to_newline(&mut self) {
        while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            self.bump();
        }
        if matches!(self.peek(), TokenKind::Newline) { self.bump(); }
    }

    fn expect_newline_or_eof(&mut self) {
        match self.peek() {
            TokenKind::Newline => { self.bump(); }
            TokenKind::Eof => {}
            _ => {
                self.diags.error(self.peek_span(), "expected newline");
                self.skip_to_newline();
            }
        }
    }
}
```

Add tests:

```rust
#[test]
fn parses_equ() {
    let toks = crate::lexer::lex(".equ FOO 42\n.equ BAR 0xFF\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    assert_eq!(lines.len(), 2);
    assert!(matches!(&lines[0], Line::Equ { name, value: 42, .. } if name == "FOO"));
    assert!(matches!(&lines[1], Line::Equ { name, value: 0xFF, .. } if name == "BAR"));
}

#[test]
fn equ_rejects_out_of_range() {
    let toks = crate::lexer::lex(".equ X 100000\n", "x.tasm").unwrap();
    let result = parse(toks);
    assert!(result.is_err());
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm equ`
Expected: PASS — both tests.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "feat(asm): parse .equ declarations"
```

---

### Task 16: Parse a slot (source -> destination, no guard, no immediate)

**Files:**
- Modify: `asm/src/parser.rs`

**Step 1: Implement**

Add a `parse_slot` method that handles the simple case first:

```rust
impl Parser {
    /// Parse one slot. Returns None and emits a diagnostic on failure.
    fn parse_slot(&mut self) -> Option<SlotSpec> {
        let start_span = self.peek_span();

        let guard = self.parse_guard()?;

        let src = self.parse_source()?;

        if !self.eat(&TokenKind::Arrow) {
            self.diags.error(self.peek_span(), "expected '->'");
            return None;
        }

        let dst = self.parse_destination()?;

        Some(SlotSpec { guard, src, dst, span: start_span })
    }

    fn parse_guard(&mut self) -> Option<Guard> {
        if !matches!(self.peek(), TokenKind::LBracket) {
            return Some(Guard::Always);
        }
        self.bump(); // [
        let inverted = self.eat(&TokenKind::Bang);
        let pname = if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump(); s
        } else {
            self.diags.error(self.peek_span(), "expected predicate name in guard");
            return None;
        };
        if pname.to_ascii_lowercase() != "p0" {
            self.diags.error(self.peek_span(), format!("unknown predicate {pname}"));
            return None;
        }
        if !self.eat(&TokenKind::RBracket) {
            self.diags.error(self.peek_span(), "expected ']'");
            return None;
        }
        Some(if inverted { Guard::IfNotP0 } else { Guard::IfP0 })
    }

    fn parse_source(&mut self) -> Option<Source> {
        if matches!(self.peek(), TokenKind::Hash) {
            self.bump();
            return self.parse_immediate();
        }
        let name = if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump(); s
        } else {
            self.diags.error(self.peek_span(), "expected source identifier or #immediate");
            return None;
        };
        match source_from_name(&name) {
            Some(s) => Some(s),
            None => {
                self.diags.error(self.peek_span(), format!("unknown source {name}"));
                None
            }
        }
    }

    fn parse_destination(&mut self) -> Option<Destination> {
        let name = if let TokenKind::Ident(s) = self.peek().clone() {
            self.bump(); s
        } else {
            self.diags.error(self.peek_span(), "expected destination identifier");
            return None;
        };
        match destination_from_name(&name) {
            Some(d) => Some(d),
            None => {
                self.diags.error(self.peek_span(), format!("unknown destination {name}"));
                None
            }
        }
    }

    fn parse_immediate(&mut self) -> Option<Source> {
        // Stub for next task — for now only literal numbers.
        match self.peek().clone() {
            TokenKind::Number(n) => {
                self.bump();
                Some(Source::Imm(ImmExpr::Literal((n as i32 & 0xFFFF) as u16)))
            }
            _ => {
                self.diags.error(self.peek_span(), "expected literal after #");
                None
            }
        }
    }
}
```

Wire `parse_slot` into `parse_program` by replacing the `_ => { self.bump(); ... }` arm with:

```rust
_ => self.parse_cycle_line(),
```

And add:

```rust
impl Parser {
    fn parse_cycle_line(&mut self) {
        let start = self.peek_span();
        let mut slots = Vec::new();
        if let Some(s) = self.parse_slot() { slots.push(s); }
        while matches!(self.peek(), TokenKind::Semi) {
            self.bump();
            if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) { break; }
            if let Some(s) = self.parse_slot() { slots.push(s); }
        }
        self.expect_newline_or_eof();
        if !slots.is_empty() {
            self.lines.push(Line::Cycle(CycleSpec { slots, span: start }));
        }
    }
}
```

Add tests:

```rust
#[test]
fn parses_simple_slot() {
    let toks = crate::lexer::lex("r0 -> r3\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    let cycle = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
    assert_eq!(cycle.slots.len(), 1);
    assert_eq!(cycle.slots[0].guard, Guard::Always);
    assert_eq!(cycle.slots[0].src, Source::Gpr(0));
    assert_eq!(cycle.slots[0].dst, Destination::Gpr(3));
}

#[test]
fn parses_immediate_literal() {
    let toks = crate::lexer::lex("#42 -> r0\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    let cycle = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
    assert_eq!(cycle.slots[0].src, Source::Imm(ImmExpr::Literal(42)));
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm parses_simple_slot parses_immediate_literal`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "feat(asm): parse a single slot (source -> destination)"
```

---

### Task 17: Parse guards `[p0]` and `[!p0]`

**Files:**
- Modify: `asm/src/parser.rs`

The guard logic is already present in Task 16's `parse_guard`. Add tests to verify.

**Step 1: Add tests**

```rust
#[test]
fn parses_guards() {
    let toks = crate::lexer::lex(
        "[p0] r0 -> r3\n[!p0] r1 -> r4\n",
        "x.tasm",
    ).unwrap();
    let lines = parse(toks).unwrap();
    let c0 = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
    let c1 = match &lines[1] { Line::Cycle(c) => c, _ => panic!() };
    assert_eq!(c0.slots[0].guard, Guard::IfP0);
    assert_eq!(c1.slots[0].guard, Guard::IfNotP0);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm parses_guards`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "test(asm): verify guard parsing for [p0] and [!p0]"
```

---

### Task 18: Parse all immediate forms (number, char, symbol)

**Files:**
- Modify: `asm/src/parser.rs`

**Step 1: Extend `parse_immediate`**

Replace the stub:

```rust
fn parse_immediate(&mut self) -> Option<Source> {
    match self.peek().clone() {
        TokenKind::Number(n) => {
            if !(n >= -32768 && n <= 65535) {
                self.diags.error(self.peek_span(),
                    format!("immediate {n} out of 16-bit range"));
            }
            self.bump();
            Some(Source::Imm(ImmExpr::Literal((n as i32 & 0xFFFF) as u16)))
        }
        TokenKind::Char(c) => {
            self.bump();
            Some(Source::Imm(ImmExpr::Literal(c)))
        }
        TokenKind::Ident(s) => {
            self.bump();
            Some(Source::Imm(ImmExpr::Symbol(s)))
        }
        _ => {
            self.diags.error(self.peek_span(), "expected literal or identifier after #");
            None
        }
    }
}
```

Add tests:

```rust
#[test]
fn parses_immediate_char() {
    let toks = crate::lexer::lex("#'A' -> r0\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
    assert_eq!(c.slots[0].src, Source::Imm(ImmExpr::Literal(b'A' as u16)));
}

#[test]
fn parses_immediate_symbol() {
    let toks = crate::lexer::lex("#loop -> r0\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
    assert_eq!(c.slots[0].src, Source::Imm(ImmExpr::Symbol("loop".into())));
}

#[test]
fn parses_immediate_negative() {
    let toks = crate::lexer::lex("#-1 -> r0\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
    assert_eq!(c.slots[0].src, Source::Imm(ImmExpr::Literal(0xFFFF))); // two's complement -1
}

#[test]
fn rejects_oversized_immediate() {
    let toks = crate::lexer::lex("#100000 -> r0\n", "x.tasm").unwrap();
    assert!(parse(toks).is_err());
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm immediate`
Expected: PASS — all four tests.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "feat(asm): parse immediates (number/char/symbol/negative)"
```

---

### Task 19: Parse multi-slot cycles separated by `;`

**Files:**
- Modify: `asm/src/parser.rs`

The multi-slot logic is already in Task 16's `parse_cycle_line`. Add tests.

**Step 1: Add tests**

```rust
#[test]
fn parses_multi_slot_cycle() {
    let toks = crate::lexer::lex("r0 -> r3; r1 -> r4; r2 -> r5\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
    assert_eq!(c.slots.len(), 3);
}

#[test]
fn allows_trailing_semicolon() {
    let toks = crate::lexer::lex("r0 -> r3;\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    let c = match &lines[0] { Line::Cycle(c) => c, _ => panic!() };
    assert_eq!(c.slots.len(), 1);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm multi_slot allows_trailing`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "test(asm): verify multi-slot cycles with ; separator"
```

---

### Task 20: Parse labels (standalone and attached to a cycle)

**Files:**
- Modify: `asm/src/parser.rs`

**Step 1: Add label parsing logic to `parse_program`**

Replace the catch-all `_ => self.parse_cycle_line()` to handle labels first:

```rust
match self.peek().clone() {
    TokenKind::Newline => { self.bump(); self.lines.push(Line::Empty); }
    TokenKind::KwEqu   => self.parse_equ(),
    TokenKind::Ident(name) if self.tokens.get(self.pos + 1).map(|t| &t.kind)
                              == Some(&TokenKind::Colon) => {
        self.parse_label(name);
    }
    _ => self.parse_cycle_line(),
}
```

Add:

```rust
impl Parser {
    fn parse_label(&mut self, name: String) {
        let span = self.peek_span();
        self.bump(); // ident
        self.bump(); // ':'
        // attached cycle on same line?
        let attached = if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            self.expect_newline_or_eof();
            None
        } else {
            // parse a cycle inline
            let start = self.peek_span();
            let mut slots = Vec::new();
            if let Some(s) = self.parse_slot() { slots.push(s); }
            while matches!(self.peek(), TokenKind::Semi) {
                self.bump();
                if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) { break; }
                if let Some(s) = self.parse_slot() { slots.push(s); }
            }
            self.expect_newline_or_eof();
            Some(CycleSpec { slots, span: start })
        };
        self.lines.push(Line::Label { name, span, attached });
    }
}
```

Add tests:

```rust
#[test]
fn parses_standalone_label() {
    let toks = crate::lexer::lex("loop:\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    assert!(matches!(&lines[0], Line::Label { name, attached: None, .. } if name == "loop"));
}

#[test]
fn parses_label_with_attached_cycle() {
    let toks = crate::lexer::lex("loop: r0 -> r3\n", "x.tasm").unwrap();
    let lines = parse(toks).unwrap();
    let l = match &lines[0] { Line::Label { name, attached: Some(c), .. } => (name, c), _ => panic!() };
    assert_eq!(l.0, "loop");
    assert_eq!(l.1.slots.len(), 1);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm label`
Expected: PASS — both tests.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "feat(asm): parse standalone and attached labels"
```

---

### Task 21: Parser error recovery

**Files:**
- Modify: `asm/src/parser.rs`

**Step 1: Verify recovery already works, add a test**

The existing `skip_to_newline` calls in error paths give us recovery. Add a test that confirms a malformed line doesn't poison the rest:

```rust
#[test]
fn recovers_after_bad_line() {
    let toks = crate::lexer::lex("r0 ->\n#42 -> r0\n", "x.tasm").unwrap();
    let result = parse(toks);
    // First line errors, second line still parses correctly. Errors are reported
    // but parsing continues — for our purposes the test is that the parser finishes.
    assert!(result.is_err());
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm recovers`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/parser.rs
git commit -m "test(asm): verify parser error recovery"
```

---

## Phase 5 — Encoder (10 tasks)

### Task 22: Encoder skeleton + symbol table

**Files:**
- Create: `asm/src/encoder.rs`
- Modify: `asm/src/lib.rs` (add `mod encoder;`)

**Step 1: Create the encoder**

`asm/src/encoder.rs`:

```rust
//! Encoder: translates parsed Lines into Vec<IWord>, resolving symbols.

use std::collections::HashMap;

use toasttta::{IWord, Slot};
use toasttta::isa::{dst, guard, src};

use crate::diag::{Diagnostics, Span};
use crate::parser::{
    CycleSpec, Destination, Guard, ImmExpr, Line, SlotSpec, Source,
};

pub fn encode(lines: Vec<Line>) -> Result<Vec<IWord>, Diagnostics> {
    let mut e = Encoder::new();
    e.run(lines);
    if e.diags.has_errors() { Err(e.diags) } else { Ok(e.iwords) }
}

struct Encoder {
    iwords: Vec<IWord>,
    symbols: HashMap<String, u16>,
    pending: Vec<Patch>,
    diags: Diagnostics,
}

struct Patch {
    addr: u16,
    slot_idx: usize,
    name: String,
    span: Span,
}

impl Encoder {
    fn new() -> Self {
        Self {
            iwords: Vec::new(),
            symbols: HashMap::new(),
            pending: Vec::new(),
            diags: Diagnostics::new(),
        }
    }

    fn run(&mut self, lines: Vec<Line>) {
        // Pass 1
        for line in lines {
            self.handle_line(line);
        }
        // Pass 2: backpatch forward references
        for patch in self.pending.drain(..).collect::<Vec<_>>() {
            match self.symbols.get(&patch.name) {
                Some(&addr) => {
                    let raw = self.iwords[patch.addr as usize].slots[patch.slot_idx].encode();
                    let mut decoded = Slot::decode(raw);
                    decoded.src_data = addr;
                    self.iwords[patch.addr as usize].slots[patch.slot_idx] = decoded;
                }
                None => self.diags.error(patch.span,
                    format!("undefined symbol '{}'", patch.name)),
            }
        }
    }

    fn handle_line(&mut self, line: Line) {
        match line {
            Line::Empty => {}
            Line::Equ { name, value, span } => {
                if self.symbols.insert(name.clone(), value).is_some() {
                    self.diags.error(span, format!("duplicate symbol '{name}'"));
                }
            }
            Line::Label { name, span, attached } => {
                let addr = self.iwords.len() as u16;
                if self.symbols.insert(name.clone(), addr).is_some() {
                    self.diags.error(span, format!("duplicate symbol '{name}'"));
                }
                if let Some(cycle) = attached {
                    self.encode_cycle(cycle);
                }
            }
            Line::Cycle(cycle) => self.encode_cycle(cycle),
        }
    }

    fn encode_cycle(&mut self, _cycle: CycleSpec) {
        // Stubbed for next task.
        self.iwords.push(IWord::new(
            Slot::nop(), Slot::nop(), Slot::nop(), Slot::nop()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    fn pipeline(src: &str) -> Result<Vec<IWord>, Diagnostics> {
        let toks = lex(src, "x.tasm").unwrap();
        let lines = parse(toks).unwrap();
        encode(lines)
    }

    #[test]
    fn empty_program() {
        let words = pipeline("").unwrap();
        assert!(words.is_empty());
    }

    #[test]
    fn equ_recorded() {
        let words = pipeline(".equ FOO 42\n").unwrap();
        assert!(words.is_empty()); // .equ produces no I-mem
    }

    #[test]
    fn duplicate_symbol_errors() {
        let result = pipeline(".equ X 1\n.equ X 2\n");
        assert!(result.is_err());
    }
}
```

Add to `asm/src/lib.rs`:

```rust
pub mod encoder;
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm -- empty_program equ_recorded duplicate_symbol`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/encoder.rs asm/src/lib.rs
git commit -m "feat(asm): encoder skeleton with symbol table and pass-2 backpatch"
```

---

### Task 23: Encode a single slot (with auto-NOP padding)

**Files:**
- Modify: `asm/src/encoder.rs`

**Step 1: Implement `encode_cycle` and `encode_slot`**

Replace the stubbed `encode_cycle`:

```rust
impl Encoder {
    fn encode_cycle(&mut self, cycle: CycleSpec) {
        let addr = self.iwords.len() as u16;
        let mut slots = [Slot::nop(); 4];
        if cycle.slots.len() > 4 {
            self.diags.error(cycle.span,
                format!("more than 4 slots in a single cycle ({})", cycle.slots.len()));
        }
        for (i, spec) in cycle.slots.iter().enumerate().take(4) {
            slots[i] = self.encode_slot(addr, i, spec);
        }
        self.iwords.push(IWord::new(slots[0], slots[1], slots[2], slots[3]));
    }

    fn encode_slot(&mut self, addr: u16, idx: usize, spec: &SlotSpec) -> Slot {
        let g = match spec.guard {
            Guard::Always   => guard::ALWAYS,
            Guard::IfP0     => guard::IF_P0,
            Guard::IfNotP0  => guard::IF_NP0,
        };

        let (src_sock, src_data) = self.encode_source(addr, idx, &spec.src);
        let dst_sock = encode_destination(&spec.dst);

        Slot::new(g, src_sock, src_data, dst_sock)
    }

    fn encode_source(&mut self, addr: u16, idx: usize, source: &Source) -> (u8, u16) {
        match source {
            Source::Gpr(n) => (*n, 0),
            Source::BrfP0  => (src::BRF_P0, 0),
            Source::AluR   => (src::ALU_R, 0),
            Source::AluP   => (src::ALU_P, 0),
            Source::LsuR   => (src::LSU_R, 0),
            Source::MulR   => (src::MUL_R, 0),
            Source::Imm(ImmExpr::Literal(v)) => (src::IMMEDIATE, *v),
            Source::Imm(ImmExpr::Symbol(name)) => {
                if let Some(&v) = self.symbols.get(name) {
                    (src::IMMEDIATE, v)
                } else {
                    self.pending.push(Patch {
                        addr, slot_idx: idx,
                        name: name.clone(),
                        span: Span { file: std::sync::Arc::from(""), line: 0, col: 0, len: 0 },
                    });
                    (src::IMMEDIATE, 0)
                }
            }
        }
    }
}

fn encode_destination(d: &Destination) -> u8 {
    match d {
        Destination::Gpr(n)     => *n,
        Destination::BrfP0      => dst::BRF_P0,
        Destination::AluA       => dst::ALU_A,
        Destination::AluAddT    => dst::ALU_ADD_T,
        Destination::AluSubT    => dst::ALU_SUB_T,
        Destination::AluAndT    => dst::ALU_AND_T,
        Destination::AluOrT     => dst::ALU_OR_T,
        Destination::AluXorT    => dst::ALU_XOR_T,
        Destination::AluShlT    => dst::ALU_SHL_T,
        Destination::AluShrT    => dst::ALU_SHR_T,
        Destination::AluSshrT   => dst::ALU_SSHR_T,
        Destination::AluEqT     => dst::ALU_EQ_T,
        Destination::AluNeT     => dst::ALU_NE_T,
        Destination::AluLtT     => dst::ALU_LT_T,
        Destination::AluLeT     => dst::ALU_LE_T,
        Destination::AluGtT     => dst::ALU_GT_T,
        Destination::AluGeT     => dst::ALU_GE_T,
        Destination::LsuLdT     => dst::LSU_LD_T,
        Destination::LsuStA     => dst::LSU_ST_A,
        Destination::LsuStT     => dst::LSU_ST_T,
        Destination::GcuJmpT    => dst::GCU_JMP_T,
        Destination::Discard    => dst::DISCARD,
        Destination::MulA       => dst::MUL_A,
        Destination::MulT       => dst::MUL_T,
    }
}
```

Add tests:

```rust
#[test]
fn encodes_single_slot_with_nop_padding() {
    let words = pipeline("r0 -> r3\n").unwrap();
    assert_eq!(words.len(), 1);
    let s = words[0].slots[0];
    assert_eq!(s.guard, guard::ALWAYS);
    assert_eq!(s.src_sock, src::GPR_R0);
    assert_eq!(s.dst_sock, dst::GPR_R3);
    // Other 3 slots should be NOPs (guard = NEVER)
    for i in 1..4 {
        assert_eq!(words[0].slots[i].guard, guard::NEVER);
    }
}

#[test]
fn encodes_immediate_literal() {
    let words = pipeline("#42 -> r0\n").unwrap();
    let s = words[0].slots[0];
    assert_eq!(s.src_sock, src::IMMEDIATE);
    assert_eq!(s.src_data, 42);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm encodes_single encodes_immediate`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/encoder.rs
git commit -m "feat(asm): encode slots and cycles with auto-NOP padding"
```

---

### Task 24: Backward and forward symbol references

**Files:**
- Modify: `asm/src/encoder.rs`

**Step 1: Add tests verifying both directions**

The encoder already supports both via `encode_source` (resolves immediately if found, else queues a patch). Add tests:

```rust
#[test]
fn backward_label_resolves() {
    let src = "loop:\n#loop -> r0\n";
    let words = pipeline(src).unwrap();
    assert_eq!(words.len(), 1); // only the cycle line; label produces no IWord
    assert_eq!(words[0].slots[0].src_data, 0); // loop is at addr 0
}

#[test]
fn forward_label_resolves() {
    let src = "#end -> r0\nend:\n";
    let words = pipeline(src).unwrap();
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].slots[0].src_data, 1); // end is at addr 1 (after the cycle)
}

#[test]
fn undefined_symbol_errors() {
    let result = pipeline("#nope -> r0\n");
    assert!(result.is_err());
}

#[test]
fn equ_symbol_resolves() {
    let src = ".equ ANS 42\n#ANS -> r0\n";
    let words = pipeline(src).unwrap();
    assert_eq!(words[0].slots[0].src_data, 42);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm label_resolve forward_label undefined_symbol equ_symbol`
Expected: PASS — all four.

**Step 3: Commit**

```bash
git add asm/src/encoder.rs
git commit -m "test(asm): verify forward/backward label and .equ resolution"
```

---

### Task 25: Validation V1 — multiple writes to same destination

**Files:**
- Modify: `asm/src/encoder.rs`

**Step 1: Add validation before encoding**

In `encode_cycle`, before encoding slots, run:

```rust
self.validate_cycle(&cycle);
```

Add:

```rust
impl Encoder {
    fn validate_cycle(&mut self, cycle: &CycleSpec) {
        // V1: detect multiple writes to the same destination.
        let mut seen: Vec<&Destination> = Vec::new();
        for slot in &cycle.slots {
            if seen.iter().any(|d| **d == slot.dst) {
                self.diags.error(slot.span.clone(),
                    format!("multiple writes to the same destination in this cycle"));
            }
            seen.push(&slot.dst);
        }
    }
}
```

Add a test:

```rust
#[test]
fn rejects_duplicate_destination() {
    let result = pipeline("r0 -> r3; r1 -> r3\n");
    assert!(result.is_err());
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm rejects_duplicate`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/encoder.rs
git commit -m "feat(asm): V1 — reject multiple writes to same destination"
```

---

### Task 26: Validation V2-V5 — multi-trigger detection

**Files:**
- Modify: `asm/src/encoder.rs`

**Step 1: Extend validation**

Inside `validate_cycle`, after the V1 loop, add:

```rust
let mut alu_triggers = 0;
let mut lsu_triggers = 0;
let mut mul_triggers = 0;
let mut gcu_triggers = 0;
for slot in &cycle.slots {
    use Destination::*;
    match slot.dst {
        AluAddT | AluSubT | AluAndT | AluOrT | AluXorT
        | AluShlT | AluShrT | AluSshrT
        | AluEqT  | AluNeT  | AluLtT  | AluLeT  | AluGtT | AluGeT => alu_triggers += 1,
        LsuLdT | LsuStT => lsu_triggers += 1,
        MulT => mul_triggers += 1,
        GcuJmpT => gcu_triggers += 1,
        _ => {}
    }
}
if alu_triggers > 1 {
    self.diags.error(cycle.span.clone(), "more than one ALU trigger in a single cycle");
}
if lsu_triggers > 1 {
    self.diags.error(cycle.span.clone(), "more than one LSU trigger in a single cycle");
}
if mul_triggers > 1 {
    self.diags.error(cycle.span.clone(), "more than one MUL trigger in a single cycle");
}
if gcu_triggers > 1 {
    self.diags.error(cycle.span.clone(), "more than one GCU jump in a single cycle");
}
```

Add tests:

```rust
#[test]
fn rejects_two_alu_triggers() {
    let result = pipeline("r0 -> ALU_ADD_T; r1 -> ALU_SUB_T\n");
    assert!(result.is_err());
}

#[test]
fn rejects_two_lsu_triggers() {
    let result = pipeline("r0 -> LSU_LD_T; r1 -> LSU_ST_T\n");
    assert!(result.is_err());
}

#[test]
fn allows_one_trigger_each_fu() {
    // Mixed FUs is fine.
    let result = pipeline("r0 -> ALU_ADD_T; r1 -> LSU_LD_T; r2 -> MUL_T\n");
    assert!(result.is_ok());
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm rejects_two_alu rejects_two_lsu allows_one_trigger`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/encoder.rs
git commit -m "feat(asm): V2-V5 — reject multiple FU triggers per cycle"
```

---

### Task 27: Resolve mandelbrot.tasm equivalence — sanity smoke test

**Files:**
- Modify: `asm/src/encoder.rs`

This is a **mid-development sanity check** — we don't yet have the full assembler, but we have enough to assemble *one* cycle and compare to a hand-encoded equivalent.

**Step 1: Add a smoke test that assembles a known-good fragment**

```rust
#[test]
fn smoke_test_known_cycle() {
    // r0 -> ALU_A; #42 -> ALU_ADD_T
    // Hand-encoded equivalent (mirrors what the example programs do):
    let expected_slot0 = Slot::new(guard::ALWAYS, src::GPR_R0, 0,  dst::ALU_A);
    let expected_slot1 = Slot::new(guard::ALWAYS, src::IMMEDIATE, 42, dst::ALU_ADD_T);

    let words = pipeline("r0 -> ALU_A; #42 -> ALU_ADD_T\n").unwrap();
    assert_eq!(words[0].slots[0], expected_slot0);
    assert_eq!(words[0].slots[1], expected_slot1);
    assert_eq!(words[0].slots[2].guard, guard::NEVER);
    assert_eq!(words[0].slots[3].guard, guard::NEVER);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm smoke_test`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/encoder.rs
git commit -m "test(asm): smoke test for hand-encoded equivalence"
```

---

## Phase 6 — Public API + CLI (3 tasks)

### Task 28: Wire up `assemble()` top-level

**Files:**
- Modify: `asm/src/lib.rs`

**Step 1: Replace the stub**

```rust
//! ToastTTA assembler library.

pub mod diag;
pub mod lexer;
pub mod parser;
pub mod encoder;

use toasttta::IWord;
use crate::diag::Diagnostics;

pub fn assemble(source: &str, filename: &str) -> Result<Vec<IWord>, Diagnostics> {
    let tokens = lexer::lex(source, filename)?;
    let lines  = parser::parse(tokens)?;
    encoder::encode(lines)
}

#[cfg(test)]
mod end_to_end {
    use super::*;

    #[test]
    fn hello_program_assembles() {
        let src = "
.equ HALT 0xFFFE
main: #3 -> ALU_A; #4 -> ALU_ADD_T
      ALU_R -> r0
      #HALT -> LSU_ST_A; r0 -> LSU_ST_T
";
        let words = assemble(src, "test.tasm").unwrap();
        assert_eq!(words.len(), 3);
    }
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm hello_program`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/lib.rs
git commit -m "feat(asm): wire up top-level assemble() function"
```

---

### Task 29: CLI binary

**Files:**
- Modify: `asm/src/main.rs`

**Step 1: Implement the CLI**

```rust
use std::fs;
use std::path::PathBuf;
use std::process;

use toasttta_asm::{assemble, diag::{Diagnostics, Severity}};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} INPUT.tasm [-o OUTPUT.bin]", args[0]);
        process::exit(2);
    }
    let input = PathBuf::from(&args[1]);
    let output = if let Some(i) = args.iter().position(|a| a == "-o") {
        PathBuf::from(args.get(i + 1).unwrap_or(&String::new()).clone())
    } else {
        input.with_extension("bin")
    };

    let source = match fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {}: {e}", input.display());
            process::exit(2);
        }
    };

    match assemble(&source, &input.to_string_lossy()) {
        Ok(words) => {
            let mut bytes = Vec::with_capacity(words.len() * 16);
            for w in &words {
                bytes.extend_from_slice(&w.encode().to_le_bytes());
            }
            if let Err(e) = fs::write(&output, &bytes) {
                eprintln!("failed to write {}: {e}", output.display());
                process::exit(2);
            }
            eprintln!("wrote {} instruction words to {}", words.len(), output.display());
        }
        Err(diags) => {
            print_diags(&diags);
            process::exit(1);
        }
    }
}

fn print_diags(d: &Diagnostics) {
    for diag in &d.items {
        let sev = match diag.severity { Severity::Error => "error", Severity::Warning => "warning" };
        eprintln!("{}: {}", sev, diag.message);
        eprintln!("  --> {}:{}:{}", diag.span.file, diag.span.line, diag.span.col);
    }
}
```

**Step 2: Verify the CLI runs**

Run: `cargo build -p toasttta-asm`
Expected: PASS.

Run: `echo '#3 -> ALU_A; #4 -> ALU_ADD_T' > /tmp/x.tasm && cargo run -p toasttta-asm --quiet -- /tmp/x.tasm -o /tmp/x.bin && ls -la /tmp/x.bin`
Expected: produces a 16-byte file (one IWord).

**Step 3: Commit**

```bash
git add asm/src/main.rs
git commit -m "feat(asm): CLI binary"
```

---

### Task 30: Better diagnostic rendering with snippet + caret

**Files:**
- Modify: `asm/src/diag.rs`

**Step 1: Add a `render` method**

```rust
impl Diagnostics {
    pub fn render(&self, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let mut out = String::new();
        for d in &self.items {
            let sev = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            out.push_str(&format!("{sev}: {}\n", d.message));
            out.push_str(&format!("  --> {}:{}:{}\n",
                d.span.file, d.span.line, d.span.col));
            if let Some(line) = lines.get(d.span.line.saturating_sub(1) as usize) {
                let line_num = format!("{:4}", d.span.line);
                out.push_str(&format!("{line_num} | {line}\n"));
                let pad = " ".repeat(line_num.len() + 3 + (d.span.col as usize - 1));
                let caret = "^".repeat(d.span.len.max(1) as usize);
                out.push_str(&format!("{pad}{caret}\n"));
            }
        }
        out
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn renders_caret_under_span() {
        let mut d = Diagnostics::new();
        d.error(Span { file: Arc::from("x.tasm"), line: 1, col: 6, len: 3 }, "oh no");
        let out = d.render("hello world\n");
        assert!(out.contains("oh no"));
        assert!(out.contains("^^^"));
    }
}
```

Update `print_diags` in `asm/src/main.rs`:

```rust
fn print_diags(d: &Diagnostics, source: &str) { eprint!("{}", d.render(source)); }
```

And in `main()`:

```rust
Err(diags) => {
    print_diags(&diags, &source);
    process::exit(1);
}
```

**Step 2: Run**

Run: `cargo test -p toasttta-asm renders_caret`
Expected: PASS.

**Step 3: Commit**

```bash
git add asm/src/diag.rs asm/src/main.rs
git commit -m "feat(asm): render diagnostics with snippet and caret"
```

---

## Phase 7 — Golden integration tests (3 tasks)

These tasks **prove the assembler is correct** by re-writing the three existing example programs as `.tasm` files and byte-comparing the output against the hand-coded Rust IWord constructions.

### Task 31: sample_prog.tasm + golden test

**Files:**
- Create: `asm/tests/golden_sample_prog.rs`
- Create: `asm/tests/fixtures/sample_prog.tasm`

**Step 1: Write the .tasm**

Create `asm/tests/fixtures/sample_prog.tasm`. It must mirror `emu/examples/sample_prog.rs` exactly. Translate the Rust IWord constructors line-by-line:

```
.equ STDOUT_CHAR 0xFF00
.equ STDOUT_INT  0xFF01
.equ HALT        0xFFFE

main:
  // banner: "ToastTTA v1.0\n" — 14 chars
  #STDOUT_CHAR -> LSU_ST_A; #'T' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'o' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'a' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'s' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'t' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'T' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'T' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'A' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #' ' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'v' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'1' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'.' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'0' -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'\n' -> LSU_ST_T

  // sum = 1+...+10 in a predicated loop
  #0 -> r0; #1 -> r1; #10 -> r2

loop:
  r1 -> ALU_A; r2 -> ALU_LE_T
  ALU_P -> p0
  [!p0] #print -> GCU_JMP_T; r0 -> ALU_A; r1 -> ALU_ADD_T
  ALU_R -> r0; r1 -> ALU_A; #1 -> ALU_ADD_T
  ALU_R -> r1; #loop -> GCU_JMP_T

print:
  #STDOUT_INT  -> LSU_ST_A; r0 -> LSU_ST_T
  #STDOUT_CHAR -> LSU_ST_A; #'\n' -> LSU_ST_T
  #HALT        -> LSU_ST_A; r0 -> LSU_ST_T
```

**Step 2: Write the golden test**

Create `asm/tests/golden_sample_prog.rs`:

```rust
//! Golden test: assemble sample_prog.tasm and compare bytes against the
//! hand-encoded `emu/examples/sample_prog.rs` output.

use std::process::Command;
use toasttta_asm::assemble;

#[test]
fn sample_prog_byte_identical() {
    // Generate the hand-coded reference binary.
    let status = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "sample_prog", "-p", "toasttta-emu"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("failed to run example");
    assert!(status.success(), "example program failed");

    // Reference binary is produced in the workspace root by the example.
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap();
    let reference = std::fs::read(workspace_root.join("prog.bin"))
        .expect("reference prog.bin not found");

    let source = include_str!("fixtures/sample_prog.tasm");
    let words = assemble(source, "sample_prog.tasm").unwrap();
    let mut assembled = Vec::with_capacity(words.len() * 16);
    for w in &words {
        assembled.extend_from_slice(&w.encode().to_le_bytes());
    }

    assert_eq!(assembled, reference,
        "assembled bytes differ from hand-coded reference");
}
```

**Step 3: Run**

Run: `cargo test -p toasttta-asm sample_prog_byte_identical`
Expected: PASS — bytes match.

**Step 4: If the test fails**, the `.tasm` source must be corrected to match the example's slot ordering exactly. The hand-coded reference is the source of truth.

**Step 5: Commit**

```bash
git add asm/tests/golden_sample_prog.rs asm/tests/fixtures/sample_prog.tasm
git commit -m "test(asm): golden byte-equivalence test for sample_prog"
```

---

### Task 32: fib.tasm + golden test

**Files:**
- Create: `asm/tests/golden_fib.rs`
- Create: `asm/tests/fixtures/fib.tasm`

**Step 1: Translate `emu/examples/fib.rs` to `.tasm`**

Read `emu/examples/fib.rs` and transcribe. Key cycles: banner ("Fibonacci:\n"), init, loop body (6 cycles using the same predicated structure as sample_prog), and halt.

The translation should be mechanical — each `IWord::new(...)` call becomes one line in `.tasm`.

**Step 2: Write the golden test**

```rust
//! Golden test: fib.tasm == fib.rs hand-encoded output.

use std::process::Command;
use toasttta_asm::assemble;

#[test]
fn fib_byte_identical() {
    let status = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "fib", "-p", "toasttta-emu"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("failed to run example");
    assert!(status.success());

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap();
    let reference = std::fs::read(workspace_root.join("fib.bin")).unwrap();

    let source = include_str!("fixtures/fib.tasm");
    let words = assemble(source, "fib.tasm").unwrap();
    let mut assembled = Vec::with_capacity(words.len() * 16);
    for w in &words {
        assembled.extend_from_slice(&w.encode().to_le_bytes());
    }

    assert_eq!(assembled, reference);
}
```

**Step 3: Run**

Run: `cargo test -p toasttta-asm fib_byte_identical`
Expected: PASS.

**Step 4: Commit**

```bash
git add asm/tests/golden_fib.rs asm/tests/fixtures/fib.tasm
git commit -m "test(asm): golden byte-equivalence test for fib"
```

---

### Task 33: mandelbrot.tasm + golden test (the killer regression)

**Files:**
- Create: `asm/tests/golden_mandelbrot.rs`
- Create: `asm/tests/fixtures/mandelbrot.tasm`

**Step 1: Translate `emu/examples/mandelbrot.rs` to `.tasm`**

This is the most demanding translation. 46 instruction words. Use the existing constants:

```
.equ STDOUT_CHAR 0xFF00
.equ HALT        0xFFFE
.equ MAX_ITER    16
.equ ESCAPE_SHR  6      // (sum >> 6) != 0 → escape

// Q12.4 fixed-point constants:
.equ CX_INIT  -36       // -2.25
.equ CY_INIT  -16       // -1.0
.equ STEP     2         // 0.125
.equ COLS     32
.equ ROWS     16
```

Then walk through the inner loop, using the labels `init`, `row`, `col`, `inner`, `in_set`, `escaped`, `inc_col`, `end_row`, `end` from the design.

**Step 2: Write the golden test (same shape as the previous two)**

```rust
#[test]
fn mandelbrot_byte_identical() {
    let status = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "mandelbrot", "-p", "toasttta-emu"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("failed to run example");
    assert!(status.success());

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap();
    let reference = std::fs::read(workspace_root.join("mandelbrot.bin")).unwrap();

    let source = include_str!("fixtures/mandelbrot.tasm");
    let words = assemble(source, "mandelbrot.tasm").unwrap();
    let mut assembled = Vec::with_capacity(words.len() * 16);
    for w in &words {
        assembled.extend_from_slice(&w.encode().to_le_bytes());
    }

    assert_eq!(assembled, reference, "mandelbrot bytes differ");
}
```

**Step 3: Run**

Run: `cargo test -p toasttta-asm mandelbrot_byte_identical`
Expected: PASS — 736 bytes byte-for-byte identical.

**Step 4: Verify end-to-end**

Run: `cargo run -p toasttta-emu -- mandelbrot.bin`
Expected: prints the 32×16 ASCII Mandelbrot.

Then assemble it via the assembler and run the result:

```bash
cargo run -p toasttta-asm -- asm/tests/fixtures/mandelbrot.tasm -o /tmp/m.bin
cargo run -p toasttta-emu -- /tmp/m.bin
```

Expected: identical Mandelbrot output.

**Step 5: Commit**

```bash
git add asm/tests/golden_mandelbrot.rs asm/tests/fixtures/mandelbrot.tasm
git commit -m "test(asm): golden byte-equivalence test for mandelbrot

This is the strongest regression we have: 46 instruction words including
nested loops, predicated branches, fixed-point math, and MMIO. If the
assembled .tasm produces the same 736 bytes as the hand-coded Rust
example, the entire assembler stack is correct."
```

---

## Definition of done

After Task 33:

- `cargo test --workspace` passes 100% (lexer + parser + encoder unit tests, plus three golden equivalence tests).
- `cargo run -p toasttta-asm -- <file>.tasm -o <file>.bin` produces a binary the emulator can run.
- Mandelbrot rendered from `.tasm` source matches Mandelbrot rendered from hand-coded Rust source byte-for-byte.
- Diagnostics include file/line/col with caret pointers.
- All ten validation rules (V1–V10) reject malformed programs.

---

## Future work (out of scope, see design doc §8)

- Macros (`call`, `ret`, `push`, `pop`, user-defined)
- Instruction-level shorthand (`add r3, r1, r2`)
- `.org` and `.byte`/`.word` data directives
- Constant arithmetic in `.equ`
- `--watch` mode
- Did-you-mean suggestions
- Linker for multi-file projects
