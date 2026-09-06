# Typed string dictionaries

RWLang provides a compute-local `Dict<String,String>` collection for small deterministic maps such as metadata, parsed key/value input, lookup tables, and intermediate request processing.

```rw
let values = dict();
set values["name"] = "RWLang";
set values["mode"] = "production";

let count = len(values);
let exists = containsKey(values, "name");
let name = values["name"];

set values = removeKey(values, "mode");
```

The type is intentionally explicit: both keys and values are `String`. There is no implicit conversion from `Int`, `F32`, or other scalar types.

## Missing keys

Indexing is strict. `values["missing"]` is a runtime input error rather than silently producing an empty string. Use `containsKey(values, key)` before indexing when absence is expected.

## Mutation

Dictionary insertion and replacement use normal collection assignment:

```rw
set values[key] = value;
```

`removeKey(dict, key)` returns a new dictionary, so use scalar assignment when the result should replace the current value:

```rw
set values = removeKey(values, key);
```

## Limits and accounting

A dictionary is limited to 4096 entries. Keys must be non-empty and at most 1024 UTF-8 bytes. Dictionary keys, values, and collection overhead count against the runtime allocation budget.

`Dict<String,String>` is compute-local. It is not a model field, database scalar, form/query binding type, or business-audit scalar.

The runtime uses deterministic key ordering internally. This avoids hash-order-dependent behaviour and keeps tests and serialized JSON stable.
