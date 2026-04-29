# Default recipe: list available commands
default:
    @just --list

# Run in host-client mode (both client and server)
run:
    cargo run

# Run with Vulkan backend (WSL2 with NVIDIA GPU)
run-vulkan:
    WGPU_BACKEND=vulkan cargo run

# Run as dedicated server
run-server *args:
    cargo run --no-default-features --features server -- server {{args}}

# Run as client connecting to a server
run-client *args:
    cargo run --no-default-features --features client -- client {{args}}

# Build for Windows (cross-compile from WSL2) and run
run-windows *args:
    cargo build --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_ahoy/game
    cp target/x86_64-pc-windows-gnu/debug/bevy_ahoy.exe /mnt/c/tmp/bevy_ahoy/game/
    cp -r assets /mnt/c/tmp/bevy_ahoy/game/
    cd /mnt/c/tmp/bevy_ahoy/game && powershell.exe -c "Start-Process 'C:\tmp\bevy_ahoy\game\bevy_ahoy.exe' {{args}}"

# Run in host-client mode on Windows
host-windows:
    cargo build --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_ahoy/host
    cp target/x86_64-pc-windows-gnu/debug/bevy_ahoy.exe /mnt/c/tmp/bevy_ahoy/host/
    cp -r assets /mnt/c/tmp/bevy_ahoy/host/
    cd /mnt/c/tmp/bevy_ahoy/host && powershell.exe -c "Start-Process 'C:\tmp\bevy_ahoy\host\bevy_ahoy.exe'"

# Run dedicated server on Windows
server-windows *args:
    cargo build --no-default-features --features server --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_ahoy/server
    cp target/x86_64-pc-windows-gnu/debug/bevy_ahoy.exe /mnt/c/tmp/bevy_ahoy/server/
    cp -r assets /mnt/c/tmp/bevy_ahoy/server/
    cd /mnt/c/tmp/bevy_ahoy/server && powershell.exe -c "Start-Process 'C:\tmp\bevy_ahoy\server\bevy_ahoy.exe' -ArgumentList 'server {{args}}'"

# Run client on Windows connecting to a server
client-windows *args:
    cargo build --no-default-features --features client --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_ahoy/client
    cp target/x86_64-pc-windows-gnu/debug/bevy_ahoy.exe /mnt/c/tmp/bevy_ahoy/client/
    cp -r assets /mnt/c/tmp/bevy_ahoy/client/
    cd /mnt/c/tmp/bevy_ahoy/client && powershell.exe -c "Start-Process 'C:\tmp\bevy_ahoy\client\bevy_ahoy.exe' -ArgumentList 'client {{args}}'"

# Build for Windows without running
build-windows:
    cargo build --target x86_64-pc-windows-gnu

# Check for compilation errors without building
check:
    cargo check

# Check all feature combinations compile
check-all:
    cargo check --no-default-features --features server
    cargo check --no-default-features --features client
    cargo check --no-default-features --features client,server
    cargo check

# Run tests
test:
    cargo test

# Smoke test: launch in host-client mode for 300 frames (~5 seconds) and auto-exit
smoke-test:
    cargo run -- --frames 300

# Check + clippy lints
lint:
    cargo clippy -- -D warnings

# Build release for Windows
release-windows:
    cargo build --release --target x86_64-pc-windows-gnu
