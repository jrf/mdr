# List available recipes
default:
    @just --list

# Build in debug mode
build:
    cargo build

# Build in release mode
release:
    cargo build --release

# Run the app
run *ARGS:
    cargo run -- {{ARGS}}

# Install to ~/.local/bin
install: release
    cp target/release/mdr ~/.local/bin/
    codesign -s - ~/.local/bin/mdr

# Uninstall from ~/.local/bin
uninstall:
    rm -f ~/.local/bin/mdr

# Remove build artifacts
clean:
    cargo clean
