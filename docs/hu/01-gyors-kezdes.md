# 1. Gyors kezdés

## 1. Készíts alkalmazáskönyvtárat

Ajánlott entrypoint: `main.rw`.

```text
myapp/
├── main.rw
├── pages.rw
└── public/
```

`main.rw`:

```rwlang
mod pages;
```

`pages.rw`:

```rwlang
page fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {
        <main><h1>Hello RWLang</h1></main>
    });
}

route home GET "/" => home;
```

A `home` rövid handlernév itt azért érvényes, mert a route és a page ugyanabban a `pages` modulban van. A `mod pages;` nem emeli be a `home` nevet a `main.rw` scope-jába. Ha a route a `main.rw`-ben lenne, explicit `pages::home` handlernév kellene.

## 2. Ellenőrizd a workspace-t

```bash
./verify.sh
```

Minimum fejlesztés közben:

```bash
cargo check --workspace
cargo test --workspace
```

## 3. Indítsd development módban

```bash
cargo run -p rwlang-server -- \
  --app /abszolut/path/myapp/main.rw \
  --listen 127.0.0.1:8080 \
  --insecure-dev-cookies
```

```bash
curl -i http://127.0.0.1:8080/
```

Az `--insecure-dev-cookies` kizárólag local developmentre való.

## 4. Production: config az elsődleges

Másold és igazítsd a `config/server.toml.sample` fájlt, majd:

```bash
./rwlang-server \
  --config /usr/local/etc/rwlang/server.toml \
  --app /srv/myapp/main.rw
```

A `--app` CLI override. Ha nem akarod CLI-ben megadni, tedd a TOML `[server]` szekciójába. A precedence: `defaults < config < CLI`.

Credentialet productionban fájlból adj át (`*_file`), ne shell argumentként és ne plaintext TOML mezőben.

Következő: [Projektstruktúra, modulok és keresőbarát URL-ek](21-modules-slugs-project-layout.md).
