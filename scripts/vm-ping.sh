#!/usr/bin/env bash
# vm-ping.sh — Start the VM daemon and verify it's responsive.
#
# Usage:
#   bash scripts/vm-ping.sh [--profile <name>]
#
# Prints "pong" on success. Safe to run repeatedly — if the daemon is already
# running this is a no-op (ensure_running detects the existing socket).
#
# --profile <name>  Use a named VM profile (isolated state dir).
#                   Default: "default" (~/.local/share/pelagos/).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

KERNEL="$REPO_ROOT/out/vmlinuz"
INITRD="$REPO_ROOT/out/initramfs-custom.gz"
DISK="$REPO_ROOT/out/root.img"
BINARY="$REPO_ROOT/target/aarch64-apple-darwin/release/pelagos"

PROFILE_ARG=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --profile)
            PROFILE_ARG=(--profile "$2")
            shift 2
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

for f in "$KERNEL" "$INITRD" "$DISK" "$BINARY"; do
    if [ ! -f "$f" ]; then
        echo "Missing: $f" >&2
        echo "Run 'bash scripts/build-vm-image.sh' and 'bash scripts/sign.sh' first." >&2
        exit 1
    fi
done

exec "$BINARY" \
    "${PROFILE_ARG[@]}" \
    --kernel "$KERNEL" \
    --initrd "$INITRD" \
    --disk   "$DISK" \
    ping
