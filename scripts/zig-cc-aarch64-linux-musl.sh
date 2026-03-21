#!/bin/sh
# CC wrapper for cross-compiling C/asm to aarch64-linux-musl via zig cc.
# cc-rs passes --target=aarch64-unknown-linux-musl which zig 0.15+ rejects.
# Filter it out; zig's own -target aarch64-linux-musl is already set.
args=""
for arg in "$@"; do
    case "$arg" in
        --target=*) ;;  # drop Rust triple — zig uses its own target format
        *) args="$args $arg" ;;
    esac
done
exec /opt/homebrew/bin/zig cc -target aarch64-linux-musl $args
