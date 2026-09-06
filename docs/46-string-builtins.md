# String builtins

RWLang provides a typed, Unicode-aware string-compute core for application logic. These operations execute inside the bounded bytecode runtime; they do not bypass instruction or allocation accounting.

## API

```rw
let cleaned = trim(text);
let left = trimStart(text);
let right = trimEnd(text);
let low = lower(cleaned);
let high = upper(cleaned);
let n = stringLen(cleaned);
let has = contains(low, "rust");
let prefix = startsWith(low, "rw");
let suffix = endsWith(low, "lang");
let rewritten = replace(low, " ", "-");
let part = substring(cleaned, 1, 3);
let first = indexOf(cleaned, "rw");
let last = lastIndexOf(cleaned, "a");
let ch = charAt("RWLang", 1);
let repeated = repeat("rw", 3);
let pieces = split(cleaned, ",");
```

Signatures:

```text
stringLen(String) -> Int
trim(String) -> String
trimStart(String) -> String
trimEnd(String) -> String
lower(String) -> String
upper(String) -> String
contains(String, String) -> Bool
startsWith(String, String) -> Bool
endsWith(String, String) -> Bool
replace(String, String, String) -> String
split(String, String) -> List<String>
substring(String, Int) -> String
substring(String, Int, Int) -> String
indexOf(String, String) -> Int
lastIndexOf(String, String) -> Int
charAt(String, Int) -> String
repeat(String, Int) -> String
```

`stringLen`, `substring`, `indexOf`, `lastIndexOf`, and `charAt` use Unicode scalar-value positions rather than UTF-8 byte offsets. `indexOf` and `lastIndexOf` return `-1` when no match exists. `substring(text, start)` returns the suffix from `start`; the three-argument form takes a character count and clips the end to the available string length. Invalid negative/out-of-range indices fail closed.

Case conversion uses Unicode-aware Rust string conversion. `replace` rejects an empty search string, and `split` rejects an empty delimiter. A split is capped at 4096 result items. `repeat` rejects negative counts and is charged against the request allocation budget before allocation.

## Resource accounting

String-producing builtins reserve output memory against the request and resource-scope allocation budgets before producing the result. Case conversion is conservatively charged for possible Unicode expansion. `replace`, `split`, and `repeat` calculate or conservatively estimate output size before allocation.

The builtins also have explicit instruction costs. They remain convenient application operations, not an unmetered path around the runtime budget.

See `examples/string-operations/main.rw` and `examples/string-list/main.rw` for runnable examples.

## Statement syntax

The examples use explicit `;` terminators because each `let` is a simple statement. A newline does not terminate a statement. See [Statement terminators](56-statement-terminators.md). `String + String` concatenation and the numeric/logical operator precedence are documented in [Numeric operators, F32 math and monotonic timing](44-math-and-timing.md).
