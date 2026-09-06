all:
	cargo build --release
	strip target/release/rwlang-server
	strip target/release/rwlang-cli

install:
	sudo install -p 0755 target/release/rwlang-server /usr/local/bin
	sudo install -p 0755 target/release/rwlang-cli    /usr/local/bin
	sudo install -p 0755 config/server.toml.sample    /usr/local/etc/rwlang/
