# Default recipe: list available commands
default:
    @just --list

# Run an example (Linux native with dynamic linking)
run example="playground":
    cargo run --example {{example}}

# Run an example with Vulkan backend (WSL2 with NVIDIA GPU)
run-vulkan example="playground":
    WGPU_BACKEND=vulkan cargo run --example {{example}}

# Build for Windows (cross-compile from WSL2) and run an example
run-windows example="playground" *args:
    cargo build --example {{example}} --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_ahoy/game
    cp target/x86_64-pc-windows-gnu/debug/examples/{{example}}.exe /mnt/c/tmp/bevy_ahoy/game/
    cp -r assets /mnt/c/tmp/bevy_ahoy/game/
    cd /mnt/c/tmp/bevy_ahoy/game && powershell.exe -c "Start-Process 'C:\tmp\bevy_ahoy\game\{{example}}.exe' {{args}}"

# Run an example in host-client mode on Windows (networking enabled)
host-windows example="playground":
    cargo build --example {{example}} --features networking --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_ahoy/host
    cp target/x86_64-pc-windows-gnu/debug/examples/{{example}}.exe /mnt/c/tmp/bevy_ahoy/host/
    cp -r assets /mnt/c/tmp/bevy_ahoy/host/
    cd /mnt/c/tmp/bevy_ahoy/host && powershell.exe -c "Start-Process 'C:\tmp\bevy_ahoy\host\{{example}}.exe'"

# Build for Windows without running
build-windows example="playground":
    cargo build --example {{example}} --target x86_64-pc-windows-gnu

# Check for compilation errors without building
check:
    cargo check

# Check all feature combinations compile
check-all:
    cargo check
    cargo check --no-default-features
    cargo check --features networking
    cargo check --all-features

# Run tests
test:
    cargo test

# Check + clippy lints
lint:
    cargo clippy -- -D warnings

# Build release for Windows
release-windows example="playground":
    cargo build --release --example {{example}} --target x86_64-pc-windows-gnu
