# pelagos-mac — Ongoing Tasks

*Last updated: 2026-03-09*

---

## Current State

Repo scaffolded. Cargo workspace created with three crates:
- `pelagos-vz` — AVF binding layer (stub, `todo!()`)
- `pelagos-guest` — vsock daemon (protocol types defined, main is `todo!()`)
- `pelagos-mac` — macOS CLI (stub, `todo!()`)

No code has been executed on macOS yet. The pilot phase has not started.

---

## Phase 0 — Pilot: Validate the Architecture

**Goal:** prove that a pure-Rust macOS binary can boot a Linux VM via
`objc2-virtualization` and round-trip a vsock command to a Rust guest daemon.

**Success criteria:**
- `pelagos-mac run alpine /bin/echo hello` prints "hello" on the macOS terminal
- No Go binary involved at any layer
- virtiofsd file sharing: a host directory is visible inside the VM

### Task 0.1 — Verify objc2-virtualization crate versions

Check current versions of `objc2`, `objc2-foundation`, `objc2-virtualization` on
crates.io. Update `pelagos-vz/Cargo.toml` with correct versions. Confirm the crate
exposes `VZVirtualMachine`, `VZLinuxBootLoader`, `VZVirtioSocketDevice`.

### Task 0.2 — Implement pelagos-vz: boot a minimal Linux VM

Implement `Vm::start()` in `pelagos-vz/src/vm.rs` using `objc2-virtualization`:

1. Create `VZLinuxBootLoader` with kernel path, initrd path, command line
2. Create `VZVirtualMachineConfiguration` with CPU count and memory
3. Add `VZVirtioBlockDeviceConfiguration` for the disk
4. Add `VZVirtioNetworkDeviceConfiguration` with NAT attachment
5. Add `VZVirtioSocketDeviceConfiguration` for vsock
6. Validate configuration, instantiate `VZVirtualMachine`
7. Start the VM on a dispatch queue; wait for started state

Reference: `Code-Hex/vz` Go bindings — `vz.go`, `virtualization_13.m` for the ObjC
call sequence.

**VM image needed:** a minimal Alpine Linux ARM64 disk image with:
- Static kernel (extract from Alpine ISO or build with buildroot)
- pelagos-guest binary at `/usr/local/bin/pelagos-guest`
- init script that starts pelagos-guest on boot
- pelagos binary at `/usr/local/bin/pelagos`

See `scripts/build-vm-image.sh` (to be written).

### Task 0.3 — Implement pelagos-guest: vsock listener

Implement `main()` in `pelagos-guest/src/main.rs`:

1. Open `AF_VSOCK` socket, bind to `VSOCK_PORT` (1024), listen
2. For each accepted connection:
   - Read newline-delimited JSON `GuestCommand`
   - Dispatch: `Ping` → write `{"pong":true}\n`; `Run` → spawn pelagos, stream output
3. For `Run`: fork `pelagos run <image> -- <args>` with the given env
4. Stream stdout/stderr as `{"stream":"stdout","data":"..."}\n`
5. Write `{"exit":<code>}\n` on process exit

Cross-compile and test inside a QEMU VM before baking into disk image.

### Task 0.4 — Implement vsock client in pelagos-mac

Implement the host-side vsock client that:
1. Connects to the Unix socket exposed by AVF for the vsock device
2. Serializes a `GuestCommand::Run` as JSON, writes with newline
3. Reads the response stream, prints stdout/stderr to the terminal
4. Returns the exit code

### Task 0.5 — Wire up the CLI

Minimal `pelagos run <image> [args...]` subcommand:
1. Boot the VM if not already running (check for PID file)
2. Connect over vsock
3. Send `GuestCommand::Run`, relay output, exit with container's exit code

### Task 0.6 — Build VM image script

`scripts/build-vm-image.sh`:
- Downloads Alpine Linux ARM64 minimal ISO
- Extracts kernel + initrd
- Creates a raw disk image, installs Alpine
- Copies pelagos-guest + pelagos binaries
- Installs startup service

---

## Phase 1 — Post-Pilot

After Phase 0 is validated:

- virtiofs bind mounts in `pelagos run -v host:container`
- Rosetta for x86_64 images
- VM lifecycle management (persistent VM, auto-boot, clean shutdown)
- `pelagos build`, `pelagos image pull` forwarded to guest
- `pelagos exec`, `pelagos ps`, `pelagos logs`
- Code signing + entitlement tooling
- Signed `.pkg` installer

---

## Notes and Risks

- `objc2-virtualization` is auto-generated from Xcode SDK headers — complete but not
  ergonomic. Expect to write thin wrapper methods around raw ObjC calls.
- The `com.apple.security.virtualization` entitlement is required. Ad-hoc signing
  works for development; check the Xcode entitlement plist format early.
- virtiofsd (host side) must be running before the VM tries to mount. Coordinate
  startup order carefully.
- The vsock device in AVF is exposed as a Unix domain socket on the host. The path is
  configured at VM creation time and must be cleaned up between runs.
- macOS 13.5+ required for full feature set (virtiofs, Rosetta, EFI boot).
  macOS 12 fallback (drop virtiofs + Rosetta) is possible but deferred.
