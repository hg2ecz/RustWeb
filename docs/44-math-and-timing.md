# Numeric operators, F32 math and monotonic timing

RWLang provides checked numeric operators and a bounded math builtin surface for application and compute code.

## Operators

```rw
let sum = 10 + 3;
let difference = 10 - 3;
let product = 10 * 3;
let quotient = 10 / 3;
let remainder = 10 % 3;

let left = 3 << 2;
let right = 24 >> 2;
let masked = 12 & 10;
let toggled = 12 ^ 10;
let combined = 12 | 3;

let allowed = ready && authorized;
let fallback = cached || fresh;
let denied = !allowed;
```

`+`, `-`, `*`, `/`, and `%` are defined for matching `Int`, `F32`, and `Decimal` operands. `String + String` remains string concatenation. Shift and bitwise operators (`<<`, `>>`, `&`, `^`, `|`) require `Int`. Logical `&&`, `||`, and `!` require `Bool`; `&&` and `||` short-circuit the right-hand side.

Operator precedence, from tighter to looser, is: unary `!`; `* / %`; `+ -`; shifts; comparisons; bitwise `&`, `^`, `|`; logical `&&`; logical `||`. Parentheses should be used whenever they make intent clearer.

Arithmetic remains checked. Integer overflow, division/remainder by zero, non-finite F32 results, and invalid shift counts fail closed instead of silently wrapping or producing invalid runtime values.

## F32 math builtins

```rw
let x = 0.5f32;
let s = sin(x);
let c = cos(x);
let root = sqrt(4.0f32);
let magnitude = abs(-3.5f32);
let natural = ln(2.0f32);
let decimal = log10(1000.0f32);
let base2 = log(8.0f32, 2.0f32);
let exponential = exp(1.0f32);
let powered = pow(2.0f32, 8.0f32);
let nearest = round(2.6f32);
let down = floor(2.6f32);
let up = ceil(2.1f32);
```

Signatures:

```text
sin(F32) -> F32
cos(F32) -> F32
sqrt(F32) -> F32
abs(Int) -> Int
abs(F32) -> F32
ln(F32) -> F32
log10(F32) -> F32
log(F32, F32) -> F32
exp(F32) -> F32
pow(F32, F32) -> F32
round(F32) -> F32
floor(F32) -> F32
ceil(F32) -> F32
toF32(Int) -> F32
```

All F32 results must remain finite. Domain errors such as `sqrt(-1.0f32)` or `ln(-1.0f32)` fail instead of creating NaN or infinity.

## Monotonic timing

```rw
let started = monotonicNanos();
// measured work
let elapsed = monotonicNanos() - started;
```

`monotonicNanos()` returns an `Int` nanosecond counter relative to a process-local monotonic origin. It is suitable for elapsed-time measurements, not timestamps. Pages whose public-cache output depends on this clock are rejected by the compiler.

Math builtins have weighted instruction-budget costs. Transcendental operations cost more than basic arithmetic so compute-heavy code remains governed by the configured resource limits.

See `examples/numeric-operators/main.rw` for a runnable operator and math example.

## Related language rules

Every `let`, `set`, and `return` shown above is a simple statement and therefore ends with `;`. Newlines inside an expression do not terminate it. See [Statement terminators](56-statement-terminators.md). String concatenation with `+` and the Unicode-aware String API are documented in [String builtins](46-string-builtins.md).
