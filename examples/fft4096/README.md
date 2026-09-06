# F32 FFT4096 benchmark

This example performs a complete in-language radix-2 FFT over 4096 `F32` samples. It does not call a native FFT library.

The input contains two deterministic tones at bins 64 and 256. For an unnormalised FFT the expected magnitudes are approximately 2048 and 1024. The page reports the measured FFT section using `monotonicNanos()` and a broad correctness window.

Run the server with an instruction budget high enough for compute-heavy code, then open `/fft4096`. The first request also warms the expression-bytecode cache, so compare repeated requests when investigating execution overhead.
