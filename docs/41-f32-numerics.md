# F32 numeric values

RWLang provides `F32` for bounded, binary floating-point computation. It is distinct from `Decimal`: use `Decimal` for exact decimal/business values and `F32` for numerical algorithms where IEEE-754 single precision is appropriate.

F32 literals are explicit and require the `f32` suffix:

```rw
let x = 1.25f32;
let y = -0.5f32;
let z = x * y + 2.0f32;
```

Unsuffixed fractional literals are rejected. Mixed arithmetic is also rejected: `1 + 0.5f32` is not implicitly converted. This keeps numeric intent visible and avoids accidental loss of precision.

Runtime `F32` values must be finite. `NaN` and positive/negative infinity are rejected at input boundaries, and arithmetic that would produce a non-finite result fails closed. Division by zero is a runtime error.

`F32` is represented in the semantic AST and bytecode VM without converting through `Decimal`. The expression VM has a dedicated `PushF32` opcode and executes `+`, `-`, `*`, `/`, and `%` directly as single-precision operations.

Typed HTTP/form/query input can use `F32`; textual input is parsed as finite decimal notation. Database model fields can also use `F32`; the current DB portability layer stores it canonically as text so all supported backends preserve the same parsing contract.

This milestone intentionally does not yet add arrays, indexing, loops, trigonometric builtins, or timers. Those are the next prerequisites for the FFT4096 benchmark.
