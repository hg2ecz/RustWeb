# F32 FFT4096 benchmark

A példa teljes, 4096 mintás radix-2 FFT-t hajt végre RWLang kódban `F32` tömbökön; nem hív natív FFT könyvtárat.

A determinisztikus bemenet a 64-es és 256-os binhez tartozó két szinuszt tartalmaz. Normalizálatlan FFT esetén a várt magnitúdók megközelítőleg 2048 és 1024. Az oldal `monotonicNanos()` segítségével csak az FFT szakaszt méri, és széles correctness tartományt is ellenőriz.

A compute-heavy példa futtatásához megfelelően magas instruction budget szükséges. Az első kérés az expression-bytecode cache-t is bemelegíti, ezért teljesítmény-összehasonlításhoz ismételt kéréseket érdemes mérni.
