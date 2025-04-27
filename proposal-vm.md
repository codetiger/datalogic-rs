# JSONLogic VM **v4** Proposal

This document specifies the fourth major rewrite of the Rust **jsonlogic** library. Version 4 replaces the current recursive evaluator with an *ahead‑of‑time compiled* **stack‑based** virtual machine (VM) that maximises throughput when **one rule is executed against very large data sets**. All dynamic allocations are handled by a single **bumpalo** arena so the evaluator itself is allocation‑free.

---

## 1  High‑Level Objectives

| Objective | Success metric |
|-----------|----------------|
| *Compile once, run millions* | ≥ 5 million evaluations / s for a 5‑term rule on an Apple M2 (single thread) |
| *Predictable latency* | p99 < 150 ns for rules ≤ 200 IR ops |
| *Zero runtime allocation* | All `Value`/stack/const data live in a bumpalo arena supplied by the caller |
| *No `unsafe`* | `#![forbid(unsafe_code)]` in all new crates |

---

## 2  Crate Layout

```text
jsonlogic-
├─ compiler   # AST → IR → byte‑code  (no_std, bumpalo)
├─ vm_stack   # stack‑based interpreter (no_std, bumpalo)
└─ core       # AST, parser, public façade (std)
```

Both `compiler` and `vm_stack` depend only on `bumpalo`, `smallvec`, and `num‑traits` so that they remain embedded‑friendly.

---

## 3  Intermediate Representation (IR)

```rust
#[repr(u8)]
pub enum OpCode {
    // load / store
    LoadConst   = 0x01,
    LoadLocal   = 0x02,
    StoreLocal  = 0x03,
    LoadVar     = 0x04,

    // arithmetic / logic
    Unary       = 0x10,
    Binary      = 0x11,
    Call        = 0x12,

    // control flow
    Jump        = 0x20,
    JumpIfFalse = 0x21,

    // termination
    Return      = 0xFF,
}

/// Fixed 32‑bit encoding  ┌── opcode ─┐┌──── imm24 ──────┐
#[derive(Copy, Clone)]
pub struct Instr(pub u32);
```

*Why fixed‑width 32 bits?*  It keeps byte‑code compact (< 1 KiB typical), supports direct indexing for jumps, and requires no variable‑length decoding.

### 3.1  Operand Stack

* `SmallVec<[Value; 8]>` holds the hot path; spills go to the same bump arena.
* `locals: SmallVec<[Value; 4]>` cache common sub‑expressions when the compiler emits `Let`.

### 3.2  Const Pool

Literal `Value`s are allocated into the caller‑provided bump arena once, stored by index, and cloned by reference only (strings are `Arc<str>` for copy‑on‑write).

---

## 4  Compilation Pipeline

```text
 JSON  ─▶  parse  ─▶  optimised AST  ─▶  lower  ─▶  Vec<Instr> + ConstPool
```

All optimisations (constant folding, dead‑branch pruning, var‑path caching) stay at **AST level**; lowering to IR is therefore 100 % mechanical and trivially testable.

---

## 5  Stack VM Interpreter

```rust
pub fn run(bytecode: &[Instr], ctx: &dyn VarLookup, arena: &Bump, consts: &[Value]) -> Value {
    let mut state = VmState::new(bytecode, ctx, arena, consts);
    loop {
        let instr = state.fetch();
        match instr.opcode() {
            OpCode::LoadConst   => state.push_const(instr.imm()),
            OpCode::LoadLocal   => state.push_local(instr.imm()),
            OpCode::StoreLocal  => state.store_local(instr.imm()),
            OpCode::Unary       => state.exec_unary(instr.imm()),
            OpCode::Binary      => state.exec_binary(instr.imm()),
            OpCode::LoadVar     => state.push_var(instr.imm()),
            OpCode::Call        => state.exec_call(instr.imm()),
            OpCode::Jump        => state.jump(instr.imm()),
            OpCode::JumpIfFalse => state.jump_if_false(instr.imm()),
            OpCode::Return      => return state.pop(),
        }
    }
}
```

`arena` is borrowed, so evaluation is still `&self` and thread‑safe when each thread uses its own arena.

---

## 6  Byte‑code Examples

### 6.1  Simple arithmetic

Rule `{"+":[3,4]}` ↓

```text
0  LoadConst 0   ; 3
1  LoadConst 1   ; 4
2  Binary    ADD
3  Return
```

Stack: `3 → 3,4 → 7`.

### 6.2  If/else with cached sub‑expression

Rule:
```json
{"if":[{"==":[{"var":"x"},10]}, {"*": [{"var":"x"},2]}, 0]}
```

Lowering (assuming `x` is path‑id 0):

```text
0  LoadVar     0      ; x
1  LoadConst   0      ; 10
2  Binary      EQ
3  JumpIfFalse 8
4  LoadVar     0      ; x (re‑used)
5  LoadConst   1      ; 2
6  Binary      MUL
7  Return
8  LoadConst   2      ; 0
9  Return
```

### 6.3  Locals introduced by compiler

Rule `{"&&":[{"<":[{"var":"a"},5]}, {"<":[{"var":"a"},3]}]}` results in:

```text
0  LoadVar     0      ; a
1  StoreLocal  0
2  LoadLocal   0
3  LoadConst   0      ; 5
4  Binary      LT
5  JumpIfFalse 14
6  LoadLocal   0
7  LoadConst   1      ; 3
8  Binary      LT
9  JumpIfFalse 14
10 LoadConst   2      ; true
11 Return
14 LoadConst   3      ; false
15 Return
```

---

## 7  Memory & Safety (Bumpalo‑only Allocations)

* Caller passes `&Bump` to both compiler *and* evaluator.
* All `Value::Arr/Obj/Str` allocate in this arena; VM never calls the global allocator.
* After each evaluation the caller can reset the arena in **O(1)**:

```rust
let arena = Bump::new();
let (prog, pool) = compile(rule_json, &arena);
for data in huge_dataset {
    let out = vm_stack::run(&prog, &data_ctx(data), &arena, &pool);
    // use `out` …
    arena.reset();
}
```

---

## 8  Testing & Benchmarking

| Layer | Tool | Purpose |
|-------|------|---------|
| Compiler diff | `insta` | snapshot `Vec<Instr>` & const pool |
| VM correctness | `proptest` | compare with serde‑jsonlogic output |
| Arena leaks | `loom` | ensure no `Value` escapes arena lifetime |
| Throughput | `criterion` | target ≥ 5 M eval/s (5‑term rule) |

---

## 9  Roadmap

1. **Week 1** – Stand‑alone `vm_stack` with hand‑written byte‑code and arena integration.
2. **Week 2–3** – Lowering pass, compile → run end‑to‑end.
3. **Week 4** – Achieve feature parity with v3; fuzz tests.
4. **Week 5** – Profiling & SIMD numeric fast paths.
5. **Week 6** – Publish `v4.0.0‑alpha` and collect community feedback.

---

### Appendix A – Disassembly legend

* `LoadConst N` – push `const_pool[N]` onto the stack.
* `LoadVar id` – push value looked up via `VarLookup`.
* `Unary tag` / `Binary tag` – apply operator from static dispatch table.
* `LoadLocal` / `StoreLocal` – access compiler‑generated temp slots.
* `Jump PC` – unconditional absolute jump.
* `JumpIfFalse PC` – pop condition; jump if false‑y.
* `Return` – pop & return top of stack.

