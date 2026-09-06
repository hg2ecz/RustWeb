# RWLang editor syntax highlighting

## Vim

Copy:

- `vim/syntax/rwlang.vim` to `~/.vim/syntax/rwlang.vim`
- `vim/ftdetect/rwlang.vim` to `~/.vim/ftdetect/rwlang.vim`

Open any `*.rw` file; `:set filetype?` should report `rwlang`.

## Midnight Commander / mcedit

MC syntax registration differs slightly by distribution. Copy `mcedit/rwlang.syntax` into the user MC syntax directory and add an entry for `*.rw` to the local `Syntax` file, for example:

```
file \\.rw$ RWLang
include rwlang.syntax
```

Typical user locations are under `~/.local/share/mc/mcedit/` or `~/.config/mc/`; use the location used by your installed MC package.
