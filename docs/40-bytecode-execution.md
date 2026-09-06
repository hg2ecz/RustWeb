# Bytecode expression execution

RWLang no longer evaluates application expressions by recursively walking the AST on every execution. The compiler still builds the typed `Program`/AST representation, but runtime expression execution now uses a small stack bytecode VM.

Current pipeline:

```text
.rw source
  -> lexer/parser/type checks
  -> Program / AST
  -> expression bytecode cache
  -> stack VM
  -> runtime values
```

The VM currently covers the existing expression language: strings, integers, booleans, enum literals, variables, model fields, `slug(...)`, and checked arithmetic/bitwise/logical operators (`+`, `-`, `*`, `/`, `%`, shifts, bitwise operators, `&&`, `||`, `!`). The bytecode uses the same instruction budget accounting as the former recursive evaluator: each emitted expression operation consumes one instruction unit. Arithmetic overflow, division/remainder by zero, invalid shift counts, invalid field access, and invalid slug conversion retain fail-closed behavior. Logical `&&` and `||` compile to bytecode short-circuit jumps, so the right-hand side is evaluated only when required.

Compiled expression bytecode is cached by structural expression identity. The cache is bounded; when its entry cap is reached it is cleared rather than growing without limit. This is an intermediate execution layer, not a JIT: no native machine code is generated yet.

The AST remains the compiler-owned semantic representation. This makes the next numeric milestones possible without tying language syntax directly to a future JIT backend: `F32`/`F64`, numeric arrays, loops, math builtins, monotonic timing, then optional native compilation can target the same bytecode semantics.
