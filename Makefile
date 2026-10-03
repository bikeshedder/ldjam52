# Release builds are fully self-contained: everything in `assets/` is embedded
# into the binary (see the `embed` feature in Cargo.toml). Keep source files
# that aren't needed at runtime (e.g. WAV masters) in `assets-src/`.

NAME := ldjam52
WINDOWS_TARGET := x86_64-pc-windows-gnu
DIST := dist

.PHONY: run build linux windows clean

# Dynamic linking speeds up incremental builds during development
run:
	cargo run --features bevy/dynamic_linking

build:
	cargo build --features bevy/dynamic_linking

linux:
	cargo build --release --features embed
	mkdir -p $(DIST)
	cp target/release/$(NAME) $(DIST)/$(NAME)

windows:
	BEVY_ASSET_PATH=$(CURDIR)/assets cross build --release --features embed --target $(WINDOWS_TARGET)
	mkdir -p $(DIST)
	cp target/$(WINDOWS_TARGET)/release/$(NAME).exe $(DIST)/$(NAME).exe

clean:
	cargo clean
	rm -rf $(DIST)
