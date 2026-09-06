# Regular expressions

RWLang provides a small, resource-bounded regular-expression API for request processing and text normalization.

```rw
let ok = regexMatch(text, "^[A-Z]{2}-[0-9]{4}$");
let normalized = regexReplace(text, "[^A-Za-z0-9]+", "-");
let captures = regexCaptures(text, "^(?P<name>[a-z]+)-(?P<id>[0-9]+)$");
```

The functions are strictly typed:

```text
regexMatch(String, String) -> Bool
regexReplace(String, String, String) -> String
regexCaptures(String, String) -> Dict<String,String>
```

`regexCaptures` returns an empty dictionary when there is no match. Successful captures are available by numeric keys (`"0"`, `"1"`, ...) and named capture keys. Optional unmatched groups are omitted. Use `containsKey` before indexing when a capture is optional.

## Safety and limits

The runtime uses Rust's `regex` engine, whose supported syntax avoids backreferences and look-around constructs that require unbounded backtracking. RWLang adds explicit limits:

- pattern: at most 4096 UTF-8 bytes;
- input: at most 1 MiB;
- replacement template: at most 16 KiB;
- at most 64 capture groups;
- generated replacement/capture data: at most 16 MiB and still charged to the normal runtime allocation budget.

Invalid patterns and limit violations are request errors; they do not panic the worker. Regex operations also have a higher instruction cost than simple scalar operations.

The API deliberately keeps compiled regex objects out of the language value model. They are implementation details, which keeps application state deterministic and leaves room for a bounded compiled-pattern cache later without changing RWLang source semantics.
