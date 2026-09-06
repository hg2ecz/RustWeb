# A-to-Z mini product list

This is the book's minimal end-to-end example application.

1. Create a secret file containing an absolute SQLite URL, for example `sqlite:///tmp/rwlang-a-z-products.db`.
2. Check the source with `rwlang-cli check main.rw`.
3. Apply the migration:
   `rwlang-cli migrate apply --dir migrations --db-url-file dev-db-url`.
4. Adjust the absolute paths in `server.toml.example` for your machine.
5. Start the server with `rwlang-server --config server.toml`.
6. Open `http://127.0.0.1:8080/`.

Do not commit the development DB URL file when it contains a real credential.

The Hungarian version is available as `README_hu.md`.
