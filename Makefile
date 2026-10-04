# Release builds are fully self-contained: everything in `assets/` is embedded
# into the binary (see the `embed` feature in Cargo.toml). Keep source files
# that aren't needed at runtime (e.g. WAV masters) in `assets-src/`.

NAME := ldjam52
WINDOWS_TARGET := x86_64-pc-windows-gnu
DIST := dist

# Passed to the build (see `build.rs`), as git isn't available inside of the
# container used by cross.
export ULU_COMMIT_COUNT := $(shell git rev-list --count HEAD)
export ULU_COMMIT_HASH := $(shell git rev-parse --short HEAD)$(shell git status --porcelain --untracked-files=no | grep -q . && echo -dirty)

.PHONY: run build linux windows clean version

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

# The version of the next build, e.g. for naming a release
version:
	@echo $$(cargo pkgid | sed 's/.*[#@]//').$(ULU_COMMIT_COUNT) $(ULU_COMMIT_HASH)

clean:
	cargo clean
	rm -rf $(DIST)
