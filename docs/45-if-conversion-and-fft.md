# Compute `if`, explicit F32 conversion, and FFT4096

RWLang compute code supports a budgeted conditional without an implicit `else` branch:

```rw
let x = 3;
if x < 10 {
    set x = x + 1;
}
```

The condition must be `Bool`. Statements inside the block use the same instruction and allocation budgets as `while` compute code. Locals declared inside the block are block-local at compile time; existing locals can be changed explicitly with `set`.

## Explicit integer to F32 conversion

`toF32(Int)` is the explicit bridge from integer control/index values to binary floating-point calculations:

```rw
let i = 64;
let phase = toF32(i) * 0.125f32;
```

There is deliberately no implicit `Int`/`F32` coercion. `toF32` is an explicit potentially lossy IEEE-754 conversion for large integers; code that requires exact integer semantics should stay in `Int`.

## FFT4096 benchmark

`examples/fft4096/main.rw` implements a full iterative radix-2 Cooley-Tukey FFT in RWLang using two `Array<F32>` buffers for real and imaginary parts. It does not delegate the transform to a native FFT library.

The deterministic input signal contains a unit-amplitude tone at bin 64 and a half-amplitude tone at bin 256. For the unnormalised transform their expected magnitudes are approximately 2048 and 1024. The example reports:

- FFT elapsed time from `monotonicNanos()`;
- magnitude at bin 64;
- magnitude at bin 256;
- a broad correctness-window result.

The timer starts after signal generation and covers bit reversal plus all FFT stages. Compute-heavy examples require a sufficiently large instruction budget. For performance experiments use repeated requests: the first request may also populate expression-bytecode caches.
