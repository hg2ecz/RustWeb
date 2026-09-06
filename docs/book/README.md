# RWLang book sources

The canonical English LaTeX book lives directly under `docs/book/`:

- `main.tex`
- `chapters/`

The Hungarian translation is kept separately under `docs/book/hu/`.

From the repository root:

```bash
make book       # canonical English edition
make book-hu    # Hungarian edition
make books      # both editions
```

Or build directly:

```bash
make -C docs/book pdf
make -C docs/book hu
```

Generated PDFs:

- `docs/book/rwlang-for-web-application-developers.pdf`
- `docs/book/hu/rwlang-webfejlesztoknek-hu.pdf`

The two editions should keep the same learning arc and technical contracts. English is canonical; Hungarian is maintained as a translation.
