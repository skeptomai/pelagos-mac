# Guest Container Execution — Namespace Joining

## The Core Rule

**Never use `pelagos exec <container> <cmd>` inside the guest daemon to run commands
inside a container.** It silently runs the command in the guest's root filesystem
instead.

**Always use `setns(2)` directly via `pre_exec`, exactly as `handle_exec_into` does.**

---

## Why `pelagos exec` Silently Fails Inside the VM

`pelagos exec` is designed for rootless containers (no root privilege, no
`CAP_SYS_ADMIN`). Its namespace-joining logic has a fundamental constraint:

**`setns(CLONE_NEWPID)` only updates `pid_for_children`.**
A subsequent `fork()` is required to actually place a new process inside the PID
namespace. `exec()` alone is insufficient.

pelagos works around this with a double-fork at container creation time (the
"intermediate process P" in container.rs). But `pelagos exec` runs in `pre_exec`
(after fork, before exec) — the correct ordering to set up the user namespace and
other namespaces, but too late to redo the PID namespace double-fork. As a result,
`pelagos exec` **always skips the PID namespace join** (see `exec.rs` line ~139:
`"exec: skipping PID namespace join (host PID namespace limitation)"`).

The process spawned by `pelagos exec` therefore runs in the guest's host PID
namespace. It also runs in the guest's root mount namespace unless the mount namespace
join succeeds — but without PID namespace, `/proc` inside the chroot is wrong, and
the command effectively runs outside the container.

This is a **known, deliberate limitation** of rootless `pelagos exec`. It is
documented in `pelagos/src/cli/exec.rs` and is unlikely to be fixed because fixing
it would require either running as root or a fundamentally different exec approach.

---

## What `handle_exec_into` Does Instead

`pelagos-guest`'s `handle_exec_into` (and the `docker cp` handlers `handle_cp_from`
/ `handle_cp_to`) bypass `pelagos exec` entirely and call `setns(2)` directly:

```rust
// 1. Get the container's PID from `pelagos ps --all`
let pid = get_container_pid(container)?;

// 2. Open /proc/<pid>/ns/{net,uts,ipc,pid,mnt} fds in the parent
let ns_fds = open_ns_fds(pid)?;

// 3. Build the command and enter namespaces in the child (after fork, before exec)
unsafe {
    cmd.pre_exec(move || {
        for &ns_fd in &ns_fds {
            call_setns(ns_fd);      // setns(2) — async-signal-safe
            libc::close(ns_fd);
        }
        Ok(())
    });
}

// 4. Spawn, run, wait; parent closes its copies of ns_fds after spawn
let result = cmd.spawn()...;
for &ns_fd in &ns_fds { libc::close(ns_fd); }
```

**Why this works when `pelagos exec` doesn't:**

- The guest daemon runs as PID 1 inside the VM, which IS root (uid 0). It has
  `CAP_SYS_ADMIN` in the initial user namespace. `setns(CLONE_NEWPID)` + fork
  succeeds because:
  - `pre_exec` runs after `fork()` in the child — we are in the right place in
    the PID namespace double-fork sequence.
  - We have the required capabilities.
- `ns_fds` is a `[i32; 5]` — a `Copy` type. The `move` closure gets a copy, the
  parent retains the originals for cleanup after spawn.
- Namespace join order matters: `[net, uts, ipc, pid, mnt]`. Mount last because
  `/proc` must remain readable until the PID join completes.

---

## Implications — What We Can and Cannot Do

### Can Do

| Operation | Mechanism |
|---|---|
| `docker exec` — interactive or piped | `handle_exec_into` — direct setns |
| `docker cp container→host` | `handle_cp_from` — direct setns + tar |
| `docker cp host→container` | `handle_cp_to` — direct setns + tar |
| Any future "run inside container" operation | Direct setns pattern |

### Cannot Do

| Operation | Why |
|---|---|
| Use `pelagos exec` subprocess from guest code | Silently skips PID ns; runs in guest root |
| Use `spawn_and_stream` for container-internal ops | Same — `spawn_and_stream` uses `pelagos` CLI |
| `pelagos exec` from VM shell (interactive use) | Works for debugging but has same PID ns gap |

### Things That Appear to Work but Are Wrong

`pelagos exec <container> cat /etc/passwd` run from the guest daemon will:
- **Succeed** (exit 0)
- **Return `/etc/passwd` from the guest's own filesystem**, not the container's

This is a silent correctness failure, not a crash. Any new guest code that runs
commands "inside containers" must be audited to ensure it uses the direct `setns`
pattern.

---

## Template for New Guest Handlers That Execute Inside a Container

```rust
fn handle_foo_in_container(
    writer: &mut impl Write,
    container: &str,
    /* ...other args... */
) -> std::io::Result<()> {
    let pid = match get_container_pid(container) {
        Ok(p) => p,
        Err(e) => {
            send_response(writer, &GuestResponse::Error { error: format!("foo: {}", e) })?;
            return Ok(());
        }
    };
    let ns_fds = match open_ns_fds(pid) {
        Ok(f) => f,
        Err(e) => {
            send_response(writer, &GuestResponse::Error { error: format!("foo: open ns fds: {}", e) })?;
            return Ok(());
        }
    };

    let mut cmd = Command::new("...");
    /* configure cmd args, stdio */
    unsafe {
        cmd.pre_exec(move || {
            for &ns_fd in &ns_fds {
                if call_setns(ns_fd) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                libc::close(ns_fd);
            }
            Ok(())
        });
    }

    let result = cmd.spawn() /* or .output() */;
    // Close parent's copies of ns_fds — always, even on error.
    for &ns_fd in &ns_fds {
        unsafe { libc::close(ns_fd) };
    }
    /* handle result, send responses */
}
```
