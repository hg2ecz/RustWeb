# A-Z mini termeklista

A konyv teljes, elejetol vegeig vezetett minimalis peldaja.

1. Keszits egy secret fajlt, amely egy abszolut SQLite URL-t tartalmaz, peldaul:
   `sqlite:///tmp/rwlang-a-z-products.db`
2. Ellenorizd a forrast: `rwlang-cli check main.rw`.
3. Alkalmazd a migraciot:
   `rwlang-cli migrate apply --dir migrations --db-url-file dev-db-url`.
4. A `server.toml.example` abszolut pathjait igazitsd a gepedhez.
5. Inditsd: `rwlang-server --config server.toml`.
6. Nyisd meg: `http://127.0.0.1:8080/`.

A dev DB URL fajlt ne commitold valodi credentiallel.
