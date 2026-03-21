#!/usr/bin/env bash
# test-label-check.sh — Verify that the rebuilt initramfs contains the
# external-rootfs label-check code in its /init script.
#
# Usage: bash scripts/test-label-check.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
INITRD="$REPO_ROOT/out/initramfs-custom.gz"
TMPDIR_CHECK="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_CHECK"' EXIT

if [[ ! -f "$INITRD" ]]; then
    echo "FAIL: $INITRD not found — run bash scripts/build-vm-image.sh first"
    exit 1
fi

gunzip -c "$INITRD" | (cd "$TMPDIR_CHECK" && cpio -id --quiet 2>/dev/null)

PASS=0
FAIL=0

check() {
    local desc="$1" pattern="$2"
    if grep -q "$pattern" "$TMPDIR_CHECK/init" 2>/dev/null; then
        echo "  PASS: $desc"
        ((PASS++))
    else
        echo "  FAIL: $desc"
        ((FAIL++))
    fi
}

echo ""
echo "=== initramfs label-check verification ==="
check "DISK_LABEL variable present"           "DISK_LABEL"
check "ubuntu-build label detected"           "ubuntu-build"
check "external rootfs pivot message"         "external rootfs"
check "switch_root to external /sbin/init"    "switch_root.*newroot.*/sbin/init"
echo ""
echo "initramfs: $INITRD"
echo "timestamp: $(stat -f '%Sm' "$INITRD")"
echo ""
echo "Results: $PASS pass, $FAIL fail"
exit $((FAIL > 0))
