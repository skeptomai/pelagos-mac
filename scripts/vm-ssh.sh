#!/usr/bin/env bash
# vm-ssh.sh — SSH into the running VM.
#
# Usage:
#   bash scripts/vm-ssh.sh [--profile <name>] [-- <ssh-extra-args>...]
#
# Options:
#   --profile <name>  Target a named VM profile (default: "default").
#   --                Everything after -- is passed directly to ssh.
#
# Example:
#   bash scripts/vm-ssh.sh -- uname -a
#   bash scripts/vm-ssh.sh -- -T "cat /etc/os-release"
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

KERNEL="$REPO_ROOT/out/vmlinuz"
INITRD="$REPO_ROOT/out/initramfs-custom.gz"
DISK="$REPO_ROOT/out/root.img"
BINARY="$REPO_ROOT/target/aarch64-apple-darwin/release/pelagos"

PROFILE_ARG=()
EXTRA_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --profile) PROFILE_ARG=(--profile "$2"); shift 2 ;;
        --)        shift; EXTRA_ARGS=("$@"); break ;;
        *)         echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

for f in "$KERNEL" "$INITRD" "$DISK" "$BINARY"; do
    if [[ ! -f "$f" ]]; then
        echo "Missing: $f" >&2
        echo "Run 'make all' first." >&2
        exit 1
    fi
done

exec "$BINARY" \
    "${PROFILE_ARG[@]}" \
    --kernel "$KERNEL" \
    --initrd "$INITRD" \
    --disk   "$DISK" \
    vm ssh "${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"}"
