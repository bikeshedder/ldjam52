# Release builds are fully self-contained: everything in `assets/` is embedded
# into the binary (see the `embed` feature in Cargo.toml). Keep source files
# that aren't needed at runtime (e.g. WAV masters) in `assets-src/`.
#
# The web build is the exception: the browser fetches the assets over HTTP, so
# only the files which are actually needed get downloaded.

NAME := ldjam52
WINDOWS_TARGET := x86_64-pc-windows-gnu
WASM_TARGET := wasm32-unknown-unknown
DIST := dist
WEB := $(DIST)/web

# Passed to the build (see `build.rs`), as git isn't available inside of the
# container used by cross.
export ULU_COMMIT_COUNT := $(shell git rev-list --count HEAD)
export ULU_COMMIT_HASH := $(shell git rev-parse --short HEAD)$(shell git status --porcelain --untracked-files=no | grep -q . && echo -dirty)

.PHONY: run build linux windows web web-tools serve clean version

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

# Everything needed to serve the game from a web server (see `make web-tools`)
web:
	cargo build --profile wasm-release --target $(WASM_TARGET)
	mkdir -p $(WEB)
	wasm-bindgen --target web --no-typescript --out-dir $(WEB) --out-name ulu \
		target/$(WASM_TARGET)/wasm-release/$(NAME).wasm
	cp -r web/. $(WEB)/
	rsync -a --delete --exclude='*.tiled-*' assets/ $(WEB)/assets/

# Installs the wasm target and the wasm-bindgen CLI matching the version in Cargo.lock
web-tools:
	rustup target add $(WASM_TARGET)
	cargo install wasm-bindgen-cli --locked --version $$(cargo pkgid wasm-bindgen | sed 's/.*[#@]//')

# Serves the web build at http://localhost:8000 for testing
serve: web
	python3 -m http.server --directory $(WEB) 8000

# The version of the next build, e.g. for naming a release
version:
	@echo $$(cargo pkgid | sed 's/.*[#@]//').$(ULU_COMMIT_COUNT) $(ULU_COMMIT_HASH)

clean:
	cargo clean
	rm -rf $(DIST)
