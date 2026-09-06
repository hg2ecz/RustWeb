# Budgeted `while` loops and explicit local assignment

RWLang supports bounded, resource-accounted compute loops for numeric work. The loop condition must be `Bool`, every condition check and body statement consumes the request instruction budget, and exhaustion fails with the normal instruction-limit error instead of allowing an unbounded worker loop.

```rw
let i = 0;
let samples = arrayF32(4096, 0.0f32);

while i < len(samples) {
    set samples[i] = 1.0f32;
    set i = i + 1;
}
```

`set` is explicit local reassignment. It preserves the variable's static type; assigning an `F32` expression to an `Int` local is a compile error. Array element assignment remains `set array[index] = value` and is bounds checked at runtime.

The comparison operators are `==`, `!=`, `<`, `<=`, `>`, and `>=`. Ordering is currently defined for `Int`, `F32`, and `Decimal`; equality is also available for `String` and `Bool`. Mixed numeric comparison is intentionally rejected, for example `1 < 2.0f32`.

Locals introduced with `let` inside a `while` body are scoped to the compute block from the compiler's point of view. Existing outer locals may be updated with `set`. Nested `while` blocks are supported and are charged to the same instruction budget.

This feature is intended for deterministic, bounded application-side computation such as numeric transforms. It does not bypass request timeouts, allocation limits, or named resource profiles.
