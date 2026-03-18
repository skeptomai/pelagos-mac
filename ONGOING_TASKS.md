# pelagos-mac — Ongoing Tasks


*Last updated: 2026-03-17*

---

## Current State

**Phase 4 (VS Code devcontainer support) largely complete.** The Docker CLI shim
covers the full devcontainer lifecycle including `docker build` (native via
`pelagos build` — no Docker Desktop or buildah), `docker cp`, volumes, and
networks. Multi-stage build support and end-to-end devcontainer features testing
remain (issues #91, #92).

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
| `pelagos vm console` (hvc0 serial) | ✅ | PR #51 |
| `pelagos vm ssh` (dropbear + ed25519 key) | ✅ | PR #52 |
| socket_vmnet (stable NAT, no degradation) | ✅ | PR #34 |
| `devcontainer up` (VS Code devcontainer CLI) | ✅ | PR #66 |
| `docker build` | ✅ | PR #70 |
| `docker volume create/ls/rm` | ✅ | PR #70 |
| `docker network create/ls/rm` | ✅ | PR #70 |
| `docker cp` (both directions) | ✅ | PR #71 |

---

## Phase 4 — VS Code Dev Container support (Epic #67)

| Subtask | Issue | Status |
|---|---|---|
| Docker CLI shim (`pelagos-docker`) | #56 | ✅ PR #62+#63 |
| Native port forwarding | #57 | ✅ PR #59 |
| glibc/Ubuntu compat | #58 | ✅ PR #61 |
| docker exec, version, info, inspect | #64 | ✅ PR #65 |
| devcontainer up smoke test | #66 | ✅ PR #66 |
| docker build (native via pelagos) | #68 | ✅ PR #70 |
| docker cp | #69 | ✅ PR #71 |
| overlayfs / linux-lts kernel | #89 | ✅ PR #90 |

| docker build multi-stage + features test | #92 | ✅ PR #94+#100 |
| VS Code full extension integration test | #91 | 🔲 |

---

## Remaining Work


### VS Code devcontainer — current state

T2 integration harness (`scripts/test-devcontainer-e2e.sh`) is built and running.
Current result: **Suite A/B/C/D: 16/16 PASS.**

All Suite C tests now pass (node v24.14.0, npm 11.9.0) with pelagos v0.53.0
(fixes exec-into ENV/PATH, issue #115) and the host-clock-sync fix
(VM clock injected via `clock.utc=` in kernel cmdline, no NTP on startup path).

### VS Code full extension integration test (#91)

Run VS Code "Reopen in Container" against a project with a `.devcontainer/`
and verify: IDE attaches, extensions install, terminal opens inside container.

**Blockers (in order):**

1. **pelagos#120** — container `/etc/hosts` not created. **CLOSED in pelagos v0.57.0.**
   `/etc/hosts` is now populated correctly.

2. **exec-into stdin BufReader fix** (pelagos-mac#103, now CLOSED) — applied in
   `pelagos-mac/src/main.rs` `exec_command` stdin thread: replaced `io::stdin().read()`
   with `libc::read(STDIN_FILENO,...)`. Committed and merged.

3. **pelagos#TBD — exec-into does not join the container's PID namespace.**
   VS Code's `resolveAuthority` runs a process scan (`aT()` function) that ends with
   `readlink /proc/self/ns/mnt 2>/dev/null`. In pelagos containers this fails (exit
   code 1) because exec-into processes run outside the container's PID namespace:
   `/proc/self` is a 0-byte dangling symlink. The shell server exec rejects on non-zero
   exit, causing `Ioe()` → `resolveAuthority()` to fail with `NotAvailable`.

   **Root cause:** `exec_into` in pelagos joins mount/net/ipc/uts namespaces but does
   not call `setns()` for `CLONE_NEWPID`. As a result, exec'd processes are invisible
   in the container's `/proc` and `/proc/self` does not resolve.

   **Reproduction:**
   ```bash
   pelagos-docker exec -i <container> /bin/sh
   # inside: ls -la /proc/self  → 0-byte dangling symlink
   # inside: ls /proc/[0-9]*    → only shows container init PID
   ```

   **Fix required in pelagos:** `exec_into` must call `setns(pid_ns_fd, CLONE_NEWPID)`
   before fork/exec, so exec'd processes appear in the container's PID namespace and
   `/proc/self` resolves correctly.

   **Impact on VS Code attach:** Without this fix, `resolveAuthority` always fails at
   T+~4s with `{"code":"NotAvailable","detail":true}`. The muxrpc, server install,
   and port forwarding layers all work correctly — this is the only remaining blocker.

### pelagos-mac — Lower priority

- **`docker volume inspect`** — `create/ls/rm` works; `inspect` not implemented.
  Bind mounts cover most real use cases so this is low priority.
- **Dynamic virtiofs shares** (#74) — current per-path shares require knowing all
  paths at VM start time; proper dynamic sharing needed for general-purpose use.
- **Signed installer** — `.pkg` for distribution. Requires Developer ID + notarization
  + `com.apple.security.virtualization` entitlement. Not yet scoped.


---

## Key Architecture Notes

- **`pelagos exec` subprocess cannot enter container namespaces** from inside the
  guest daemon — it silently runs in the guest root instead. Always use direct
  `setns(2)` via `pre_exec`. See `docs/GUEST_CONTAINER_EXEC.md`.
- **VM networking:** socket_vmnet, subnet `192.168.105.x`, gateway `192.168.105.1`.
  Homebrew socket path: `/opt/homebrew/var/run/socket_vmnet` (no `.shared` suffix).
- **`pelagos build` uses `--network pasta`** inside the VM. `pasta` (userspace
  TCP/UDP proxy) is staged into the initramfs. Bridge/veth kernel modules are not
  required. RUN steps that need network access work via pasta.
- **`pelagos network create` requires `--subnet <CIDR>`** explicitly; the shim
  auto-generates `10.88.<hash>.0/24` from the network name.
- **Network names max 12 chars** — bridge device name is `rm-<name>`, IFNAMSIZ=15.

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
