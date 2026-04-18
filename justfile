# Default recipe: list available commands
default:
    @just --list

# Run the game (Linux native)
run:
    cargo run

# Run the game with Vulkan backend (WSL2 with NVIDIA GPU)
run-vulkan:
    WGPU_BACKEND=vulkan cargo run

# Build for Windows (cross-compile from WSL2) and run
run-windows *args:
    cargo build --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_ahoy/game
    cp target/x86_64-pc-windows-gnu/debug/bevy_ahoy.exe /mnt/c/tmp/bevy_ahoy/game/
    cp -r assets /mnt/c/tmp/bevy_ahoy/game/
    cd /mnt/c/tmp/bevy_ahoy/game && powershell.exe -c "Start-Process 'C:\tmp\bevy_ahoy\game\bevy_ahoy.exe' {{args}}"

# Run in host-client mode on Windows (networking enabled)
host-windows:
    cargo build --features networking --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_ahoy/host
    cp target/x86_64-pc-windows-gnu/debug/bevy_ahoy.exe /mnt/c/tmp/bevy_ahoy/host/
    cp -r assets /mnt/c/tmp/bevy_ahoy/host/
    cd /mnt/c/tmp/bevy_ahoy/host && powershell.exe -c "Start-Process 'C:\tmp\bevy_ahoy\host\bevy_ahoy.exe'"

# Build for Windows without running
build-windows:
    cargo build --target x86_64-pc-windows-gnu

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
release-windows:
    cargo build --release --target x86_64-pc-windows-gnu
