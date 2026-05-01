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
    mkdir -p /mnt/c/tmp/bevy_game/game
    cp target/x86_64-pc-windows-gnu/debug/bevy_game.exe /mnt/c/tmp/bevy_game/game/
    cp -r assets /mnt/c/tmp/bevy_game/game/
    cd /mnt/c/tmp/bevy_game/game && powershell.exe -c "Start-Process 'C:\tmp\bevy_game\game\bevy_game.exe' {{args}}"

# Run in host-client mode on Windows
host-windows:
    cargo build --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_game/host
    cp target/x86_64-pc-windows-gnu/debug/bevy_game.exe /mnt/c/tmp/bevy_game/host/
    cp -r assets /mnt/c/tmp/bevy_game/host/
    cd /mnt/c/tmp/bevy_game/host && powershell.exe -c "Start-Process 'C:\tmp\bevy_game\host\bevy_game.exe'"

# Run dedicated server on Windows
server-windows *args:
    cargo build --no-default-features --features server --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_game/server
    cp target/x86_64-pc-windows-gnu/debug/bevy_game.exe /mnt/c/tmp/bevy_game/server/
    cp -r assets /mnt/c/tmp/bevy_game/server/
    cd /mnt/c/tmp/bevy_game/server && powershell.exe -c "Start-Process 'C:\tmp\bevy_game\server\bevy_game.exe' -ArgumentList 'server {{args}}'"

# Run client on Windows connecting to a server
client-windows *args:
    cargo build --no-default-features --features client --target x86_64-pc-windows-gnu
    mkdir -p /mnt/c/tmp/bevy_game/client
    cp target/x86_64-pc-windows-gnu/debug/bevy_game.exe /mnt/c/tmp/bevy_game/client/
    cp -r assets /mnt/c/tmp/bevy_game/client/
    cd /mnt/c/tmp/bevy_game/client && powershell.exe -c "Start-Process 'C:\tmp\bevy_game\client\bevy_game.exe' -ArgumentList 'client {{args}}'"

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
