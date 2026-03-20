#!/usr/bin/env bash
# build-vm-start.sh — Start the build VM and wait for SSH to be ready.
#
# The build VM runs Ubuntu systemd, not pelagos-guest, so `pelagos ping`
# (which expects a vsock pong) hangs.  This script starts the VM daemon
# via `vm ssh` (which calls ensure_running() without a vsock round-trip)
# and retries until openssh-server is up.
#
# Usage:
#   bash scripts/build-vm-start.sh [--profile <name>]
#
# After this returns, connect with:
#   pelagos [--profile <name>] vm ssh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
BINARY="$REPO_ROOT/target/aarch64-apple-darwin/release/pelagos"

PROFILE="build"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --profile) PROFILE="$2"; shift 2 ;;
        *) echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

PELAGOS_BASE="${XDG_DATA_HOME:-$HOME/.local/share}/pelagos"
if [[ "$PROFILE" == "default" ]]; then
    STATE_DIR="$PELAGOS_BASE"
else
    STATE_DIR="$PELAGOS_BASE/profiles/$PROFILE"
fi

# Stop any stale daemon for this profile.
if [[ -f "$STATE_DIR/vm.pid" ]]; then
    OLD_PID=$(cat "$STATE_DIR/vm.pid" 2>/dev/null || true)
    if [[ -n "$OLD_PID" ]] && kill -0 "$OLD_PID" 2>/dev/null; then
        echo "  stopping stale daemon (pid $OLD_PID)..."
        kill -TERM "$OLD_PID" 2>/dev/null || true
        sleep 2
        kill -KILL "$OLD_PID" 2>/dev/null || true
    fi
    rm -f "$STATE_DIR/vm.pid" "$STATE_DIR/vm.sock"
fi

echo "--- starting build VM (profile: $PROFILE) ---"
echo "    daemon will start on first SSH attempt; waiting for openssh-server..."
echo ""

# Retry loop: each call starts the daemon (if not running) then tries SSH.
# Ubuntu + systemd can take 30-90s to get SSH up on first boot.
MAX=60
for i in $(seq 1 $MAX); do
    if "$BINARY" --profile "$PROFILE" vm ssh -- true 2>/dev/null; then
        echo ""
        echo "=== build VM SSH ready (profile: $PROFILE) ==="
        echo ""
        echo "Connect with:"
        echo "  pelagos --profile $PROFILE vm ssh"
        exit 0
    fi
    printf "\r  waiting for SSH... %d/%d" "$i" "$MAX"
    sleep 5
done

echo ""
echo "FAIL: SSH did not become available after $((MAX * 5))s" >&2
echo "      Check console: pelagos --profile $PROFILE vm console" >&2
exit 1
