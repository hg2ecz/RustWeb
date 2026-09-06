# F32 arrays

RWLang provides a bounded numeric array primitive for compute-heavy code:

```rw
let samples = arrayF32(4096, 0.0f32);
set samples[0] = 1.0f32;
let first = samples[0];
let n = len(samples);
```

`Array<F32>` is a runtime numeric container, not a database/model field type. Array allocation is charged against the request allocation budget. The current hard safety ceiling is 1,048,576 elements per array. Indexing is bounds checked; negative and out-of-range indexes fail closed.

The explicit `set` statement makes mutation visible to the compiler and runtime budget system. Arrays are represented compactly as F32 values rather than generic boxed values. This is the basis for the upcoming loop, math-builtin, timer and FFT4096 work.
