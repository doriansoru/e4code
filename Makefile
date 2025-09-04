all: linux deb

.PHONY: linux deb

linux:
	cargo build --release
	@echo "Linux executable is located at: target/release/e4code"

deb:
	cargo deb
	@echo "Debian package is located at: target/debian/e4code_*.deb"
