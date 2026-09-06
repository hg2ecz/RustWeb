# Typed string lists

RWLang now has a compute-local `List<String>` value. The first constructor is `split(text, delimiter)`.

```rw
let parts = split("alpha,beta,gamma", ",");
let count = len(parts);
let first = parts[0];
```

`split` has the type:

```text
split(String, String) -> List<String>
```

The delimiter must not be empty. A split is capped at 4096 resulting items and the resulting strings plus collection overhead count against the runtime allocation budget.

`List<String>` is currently a compute-local type. It is not accepted as a model field, route/form scalar, database scalar, or audit scalar. This keeps persistence and request schemas explicit while the general collection model is being built.

Indexing is checked at runtime. Negative or out-of-range indices fail closed instead of returning an implicit null value.

The collection expression layer is shared with `Array<F32>`:

```rw
let samples = arrayF32(4096, 0.0f32);
let n = len(samples);
let x = samples[0];

let words = split("one two three", " ");
let m = len(words);
let word = words[0];
```

This shared `len(collection)` and `collection[index]` representation is the base for later typed collections and dictionaries.

See `examples/string-list/main.rw` for a runnable example.
