# pelagos-mac — Ongoing Tasks

*Last updated: 2026-03-11, SHA 60c9b83 (post-PR #50)*

---

## Current State

**Phase 2 COMPLETE.** The full container lifecycle works end-to-end on real hardware.
All 16 e2e tests pass (`bash scripts/test-e2e.sh`).

### What works today

| Feature | Status | Merged |
|---|---|---|
| VM boot via AVF | ✅ | Phase 0 |
| vsock round-trip (ping/pong) | ✅ | Phase 0 |
| `pelagos run` (pull + exec) | ✅ | PR #18 |
| Persistent daemon (warm reuse) | ✅ | PR #27 |
| virtiofs bind mounts (`-v`) | ✅ | PR #28 |
| `pelagos exec` (piped + PTY) | ✅ | PR #38 |
| `pelagos ps / logs / stop / rm` | ✅ | PR #37 |
| `pelagos run --detach --name` | ✅ | PR #37 |
| `pelagos vm shell` | ✅ | PR #45 |
| Busybox applet symlinks in VM | ✅ | PR #47 |
| Persistent OCI image cache (`/dev/vda` ext2) | ✅ | PR #50 |
| ECR Public test image (no rate limits) | ✅ | PR #50 |
| devpts mount + PTY job control | ✅ | PR #38/#40 |

---

## Phase 3 — VM Access (Epic #41)

Three options for direct VM access beyond `pelagos vm shell` (vsock-based shell):

### Option A — `pelagos vm shell` (vsock) ✅ DONE (PR #45)

Interactive `/bin/sh` inside the VM over vsock. No container namespaces.
TTY and non-TTY modes both work.

### Option B — hvc0 serial console (issue #43) ← NEXT

Wire AVF's `VZVirtioConsoleDeviceSerialPortConfiguration` to the host terminal.
Lets you watch raw boot output and drop into a login prompt.
No guest changes needed — the kernel already writes to hvc0.

### Option C — SSH (issue #44)

Run `dropbear` (small sshd) inside the VM. Requires socket_vmnet or port-forward
to reach the VM from the host. Deferred until socket_vmnet is done.

---

## Phase 3 — NAT Reliability (issue #26)

`VZNATNetworkDeviceAttachment` (InternetSharing / bridge100) degrades after several
VM lifecycles: ICMP survives but all TCP fails. Recovery: `sudo pfctl -f /etc/pf.conf`.

**Root fix: migrate to socket_vmnet** (Apache 2.0, no restricted entitlement).
- socket_vmnet runs as a privileged helper; guest uses `virtio-net` as normal
- Eliminates the PF/InternetSharing dependency entirely
- Also unblocks Option C (SSH) by giving the VM a stable reachable IP

Stress test script: `scripts/test-nat-stress.sh 40`

---

## Phase 3 — Signed Installer (not yet tracked)

`.pkg` installer for distribution. Requires:
- Developer ID Application signature + notarization
- Hardened runtime entitlement
- `com.apple.security.virtualization` in the signed entitlements

---

## Build Reference

| Step | Command |
|---|---|
| Host binary | `cargo build -p pelagos-mac --release` |
| Guest (cross) | `RUSTFLAGS="-C link-self-contained=no" RUSTC="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc" cargo build -p pelagos-guest --target aarch64-unknown-linux-musl --release` |
| VM image | `bash scripts/build-vm-image.sh` |
| Code-sign | `codesign --sign - --entitlements pelagos-mac/entitlements.plist --force target/aarch64-apple-darwin/release/pelagos` |
| All tests | `bash scripts/test-e2e.sh` |
| Cold-start test | `bash scripts/test-e2e.sh --cold` |
| Interactive container | `bash scripts/test-interactive.sh` |
| VM shell | `bash scripts/vm-shell.sh` |
