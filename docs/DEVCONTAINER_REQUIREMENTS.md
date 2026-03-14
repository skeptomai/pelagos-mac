# pelagos-mac — devcontainer Support Requirements

*Authoritative requirements for VS Code Remote-Containers support via `pelagos-docker`.*

---

## How to Use This Document

Every requirement has an ID (`R-VM-*`, `R-SH-*`, `R-DC-*`). Each maps to one or
more test cases in `scripts/test-devcontainer-shim.sh` (prefix `TC-*`). A requirement
is **met** only when all its test cases pass and have been verified against a live VM.

---

## Layer Terminology

| Layer | What | Owned By |
|---|---|---|
| **VM layer** | AVF virtual machine, kernel, init, networking | pelagos-mac (this repo) |
| **Runtime layer** | pelagos guest daemon, container lifecycle | pelagos (separate repo) |
| **Shim layer** | pelagos-docker Docker CLI compatibility shim | pelagos-mac (this repo) |
| **devcontainer** | devcontainer CLI + VS Code Remote-Containers | Third-party |

---

## Part 1 — VM/Runtime Requirements

These are contracts that the VM and the pelagos runtime must uphold for the shim to work.

### R-VM-01 — VM stays up while any container is running

**Description:** The VM daemon must not exit while one or more containers are alive
(status: running). It may exit only when explicitly told to (`pelagos vm stop`) or when
the last container has exited AND a configurable idle timeout has elapsed.

**Rationale:** devcontainer CLI sends `docker run` then, milliseconds later, `docker ps`
and `docker inspect`. If the VM shuts down between commands, those calls fail with no
diagnostic.

**Acceptance Criteria:**
- After `pelagos run --detach ...`, the VM daemon is still running
- Consecutive `pelagos inspect`, `pelagos ps`, `pelagos exec-into` commands all succeed
  within 2 seconds of `run` returning
- The VM does NOT shut down between `docker run` and the subsequent `docker ps` in a
  devcontainer session

**Current State:** PARTIAL — keepalive hack (`while sleep 1000; do :; done`) appended
to the probe container command keeps the container alive so the VM stays up. This is a
shim-layer workaround for a runtime-layer gap.

**Real Fix Target:** pelagos issue #91 (exec-into preserved rootfs) + issue #93 (VM
stays up while containers exist).

---

### R-VM-02 — Container state persists after exit; discoverable via `ps --all`

**Description:** After a container exits (any exit code), its metadata (name, image,
labels, mounts, status) must be readable via `pelagos ps --all` and
`pelagos inspect <name>` until explicitly removed.

**Rationale:** devcontainer calls `docker ps -q -a --filter label=...` after the probe
run exits. If the container record is gone, devcontainer cannot find the container it
just created and reports failure.

**Acceptance Criteria:**
- `pelagos ps --all` includes exited containers
- `pelagos inspect <name>` succeeds for exited containers
- `inspect` output includes `spawn_config.labels` (or top-level `labels`) with the
  original labels passed to `run --label`
- State persists across a VM restart (state file is on disk, not in-memory only)

**Current State:** MEETS — pelagos persists container state in `~/.local/share/pelagos/`
state files. Labels present in `pelagos inspect` JSON output.

---

### R-VM-03 — Label filtering round-trip

**Description:** Labels passed to `pelagos run --label KEY=VALUE` must be readable back
via `pelagos inspect` (at key `labels` or `spawn_config.labels`) and must match exactly
(key and value, case-sensitive).

**Rationale:** devcontainer uses two labels to identify its container:
- `devcontainer.local_folder=<absolute_workspace_path>`
- `devcontainer.config_file=<absolute_devcontainer_json_path>`

The shim client-side filters `pelagos ps --all` output using these labels. If a label
is lost or transformed, the filter returns empty.

**Acceptance Criteria:**
- `pelagos run --label foo=bar ... && pelagos inspect <name>` → `labels.foo == "bar"`
- Labels survive container restart (`pelagos start`)
- Label values containing `/` and `.` (path-like values) are stored verbatim
- Multiple labels on the same container all round-trip correctly

**Current State:** MEETS — confirmed via live inspect of `test-label` container.

---

### R-VM-04 — exec-into enters container namespaces

**Description:** `pelagos exec-into <name> <cmd>` must run `<cmd>` inside the
container's mount namespace (minimum). The process must see the container's rootfs,
not the host/VM root.

**Acceptance Criteria:**
- `exec-into <name> cat /etc/os-release` returns the container's OS, not Alpine
- `exec-into <name> uname -s` returns `Linux`
- Interactive stdin/stdout work (exec with `-i` flag passes stdin to the process)
- Non-zero exit code from the command is propagated as the exit code of exec-into

**Current State:** MEETS — verified in test phase 6.

**Known Limitation:** exec-into does NOT enter the PID namespace. `/proc/self/environ`
is therefore unavailable inside exec'd processes. devcontainer CLI's fallback
(`printenv`) must work instead.

---

### R-VM-05 — Named volumes backed by persistent storage

**Description:** `pelagos volume create <name>` must create a named volume whose
contents persist across VM restarts. VS Code stores its server installation and
extensions in named volumes (`vscode`, `vscode-server-<hash>`).

**Acceptance Criteria:**
- `volume create vscode` → exists after `pelagos vm stop && pelagos run ...`
- Files written to `/root/.vscode-server` (mounted from named volume) survive VM restart
- `volume ls` includes the volume name
- `volume rm <name>` removes it permanently

**Current State:** PARTIAL — `volume create/ls/rm` delegate to pelagos. Backing store
persistence across VM restart is UNVERIFIED (issue #93).

---

### R-VM-06 — Concurrent VM command handling

**Description:** The VM daemon must handle at least 2 concurrent vsock connections
without deadlock. devcontainer CLI sometimes issues background `events` polling
concurrently with foreground `ps`/`inspect` calls.

**Acceptance Criteria:**
- `docker events` (blocking, polling) can run in background while `docker ps` succeeds
- No vsock connection backlog stalls under normal devcontainer CLI load

**Current State:** MEETS — guest daemon uses thread-per-connection accept loop.

---

## Part 2 — Shim (pelagos-docker) Requirements

### R-SH-01 — Pre-flight commands return expected formats

| Command | Expected output |
|---|---|
| `docker -v` | Any string with "docker", "version", or "pelagos" |
| `docker version` | JSON with `"Client"` and `"Server"` keys |
| `docker version --format '{{.Server.Version}}'` | Bare version matching `[0-9]+\.[0-9]+` |
| `docker context ls --format '{{json .}}'` | JSON object with `"Name": "default"` |
| `docker context show` | `default\n` |
| `docker buildx version` | Exit non-zero (signals no BuildKit; triggers fallback) |
| `docker info` | JSON with `"ServerVersion"` and `"OSType": "linux"` |

**Acceptance Criteria:** Each command returns the expected format and exit code.

---

### R-SH-02 — Probe run detection and keepalive injection

**Description:** When devcontainer CLI sends:
```
docker run --sig-proxy=false ... <image> /bin/sh -c "echo Container started"
```
the shim must:
1. Detect this as the probe pattern
2. Inject `; while sleep 1000; do :; done` after the echo
3. Force `--detach`
4. Run the container, suppress pelagos output, print exactly `Container started\n`
5. Return exit code 0

**Rationale:** This keeps the container alive so subsequent `exec` calls can enter its
namespaces. Without this, the container exits in milliseconds, its PID is reused, and
exec enters the wrong namespace.

**Acceptance Criteria:**
- Stdout is exactly `Container started\n`
- Exit code is 0
- Container is running after the command returns
- `docker ps` immediately after shows the container

**Detection rule:** `--sig-proxy=false` AND any arg contains `echo Container started`.

---

### R-SH-03 — `docker ps` label filtering

**Description:** `docker ps [-q] [-a] --filter label=KEY=VALUE [--filter label=K2=V2]`
must return only containers whose labels satisfy ALL filters (AND semantics).

**Acceptance Criteria:**
- A container run with `--label k=v` appears when filtered with `--filter label=k=v`
- A container without that label does NOT appear
- Multiple `--filter label=` clauses are ANDed
- Label values containing `/` (path separators) and `.` (dots) match exactly
- Works for running AND exited containers when `-a` is given

---

### R-SH-04 — `docker inspect` container format

**Description:** `docker inspect [--type container] <name>` must return a JSON array
with one element containing at minimum:

```json
[{
  "Id": "<name>",
  "Name": "/<name>",
  "Created": "<ISO-8601>",
  "State": {
    "Running": <bool>,
    "Status": "<running|exited|...>",
    "StartedAt": "<ISO-8601>"
  },
  "Config": {
    "Image": "<image-ref>",
    "Labels": { "key": "value", ... },
    "User": "",
    "Env": ["K=V", ...],
    "Cmd": [],
    "WorkingDir": "",
    "Entrypoint": null
  },
  "HostConfig": { "Binds": ["<host>:<container>", ...] },
  "Mounts": [
    { "Type": "bind", "Source": "<host-path>", "Destination": "<container-path>",
      "Mode": "", "RW": true, "Propagation": "rprivate" }
  ],
  "NetworkSettings": { "Ports": {} }
}]
```

**Critical fields devcontainer reads:**
- `State.Running` — used to decide whether to `start` or `exec`
- `Config.Labels` — used to identify the container across sessions
- `Mounts[].Source` — must be **host** paths (not VM-internal `/mnt/share0/...`)
- `Created` — used as a lifecycle marker for idempotent setup scripts
- `Config.Env` — must be an array, not an object

**Acceptance Criteria:**
- All required fields present with correct types
- Mount `Source` values are host paths (translation via `vm.mounts` applied)
- Labels match what was passed to `run --label`
- `Config.Env` is a JSON array of `"K=V"` strings

---

### R-SH-05 — Mount path translation

**Description:** When pelagos stores a bind mount as `/mnt/share0/Projects/foo:/workspace`,
the shim must translate it to `/Users/cb/Projects/foo:/workspace` in all inspect output.

**Acceptance Criteria:**
- `docker inspect` bind mounts show host paths
- Translation uses `~/.local/share/pelagos/vm.mounts` (written at VM boot)
- Unmapped paths are returned as-is (not silently dropped)

---

### R-SH-06 — `docker exec` interactive mode

**Description:** `docker exec [-i] [-t] [-u <user>] [-e K=V] <name> <cmd> [args...]`
must run `<cmd>` inside the container.

**Acceptance Criteria:**
- Non-interactive: stdout of command returned on stdout, exit code propagated
- Interactive (`-i`): stdin is forwarded to the subprocess
- Exit code of the command is the exit code of `docker exec`
- `-u root` flag accepted (silently passed through or enforced)
- `-e K=V` sets env vars in the exec'd process

---

### R-SH-07 — `docker build` delegates to pelagos build

**Description:** `docker build -t <tag> [-f <Dockerfile>] <context>` must:
1. Archive the build context and transfer it to the VM
2. Pre-pull all `FROM` base images that are not prior-stage aliases
3. Call `pelagos build -t <tag> -f <Dockerfile> <context-in-vm>`
4. Stream build output to stderr
5. Return exit 0 on success, non-zero on failure

**Acceptance Criteria:**
- Single-stage Dockerfile: image available for `docker run` after build
- Multi-stage Dockerfile (`FROM x AS stage1` + `COPY --from=stage1`): succeeds
- Correct base images are pre-pulled (stage aliases are NOT pulled as images)
- Build output is streamed in real time (not buffered)

**Current State:** PARTIAL — implemented in `pelagos-guest/src/main.rs handle_build`.
Multi-stage pre-pull logic is code-complete but END-TO-END UNTESTED (issue #92).

---

### R-SH-08 — `docker volume` CRUD

| Command | Expected |
|---|---|
| `volume create <name>` | Prints `<name>`, exit 0 |
| `volume ls` | Lists volume names, one per line |
| `volume ls -q` | Same, quiet (no header) |
| `volume rm <name>` | Exit 0 on success |
| `volume inspect <name>` | JSON with `Name`, `Mountpoint` — LOW PRIORITY |

---

### R-SH-09 — `docker network` CRUD

| Command | Expected |
|---|---|
| `network create <name>` | Prints network ID, exit 0 |
| `network ls` | Lists networks |
| `network rm <name>` | Exit 0 on success |
| `network inspect <name>` | JSON — LOW PRIORITY |

---

### R-SH-10 — `docker events` polling

**Description:** `docker events [--filter event=start]` must block and emit synthetic
Docker event JSON as new containers appear. devcontainer CLI runs this in the background
as a liveness check.

**Acceptance Criteria:**
- Does not exit immediately
- Emits a `start` event JSON when a new container starts
- JSON format: `{"Type":"container","Action":"start","Actor":{"ID":"<name>","Attributes":{...}}}`
- Does not emit events for containers that existed before the call
- Terminates cleanly when the caller sends SIGTERM/SIGKILL

---

## Part 3 — devcontainer CLI Integration Requirements

These are end-to-end requirements for a complete devcontainer session.

### R-DC-01 — `devcontainer up` succeeds with a pre-built image

**devcontainer.json:**
```json
{ "image": "mcr.microsoft.com/devcontainers/base:ubuntu" }
```

**Acceptance Criteria:**
- `devcontainer up --workspace-folder <path>` exits 0
- `outcome` in the JSON result is `"success"`
- `devcontainer exec` works inside the container afterwards
- Container has correct labels and mounts

---

### R-DC-02 — `devcontainer up` succeeds with a custom Dockerfile

**devcontainer.json:**
```json
{
  "build": { "dockerfile": "Dockerfile", "context": ".." },
  "workspaceMount": "source=${localWorkspaceFolder},target=/workspace,type=bind"
}
```

**Acceptance Criteria:**
- `docker build` is called with the devcontainer.json's `dockerfile`/`context`
- Image is built successfully inside the VM
- Container is started from the built image
- `devcontainer up` exits 0 with `"outcome": "success"`

**Current State:** UNTESTED (issue #92).

---

### R-DC-03 — `devcontainer up` succeeds with devcontainer features

**devcontainer.json:**
```json
{
  "image": "mcr.microsoft.com/devcontainers/base:ubuntu",
  "features": {
    "ghcr.io/devcontainers/features/node:1": {}
  }
}
```

**Acceptance Criteria:**
- devcontainer CLI builds the feature layer Dockerfile
- Multi-stage or layered build completes inside the VM
- `node --version` works inside the container after `devcontainer up`

**Current State:** UNTESTED (issue #92).

---

### R-DC-04 — Container survives `devcontainer exec` after `devcontainer up`

**Acceptance Criteria:**
- `devcontainer exec --workspace-folder <path> -- node --version` works after R-DC-03
- exec enters the correct container (not a different namespace)
- Exit code of the exec'd command is propagated

---

### R-DC-05 — VS Code Remote-Containers full IDE flow

**Acceptance Criteria:**
- "Reopen in Container" opens the container
- VS Code Server installs into the `vscode` named volume
- Terminal opens inside the container
- Extensions listed in `devcontainer.json` install
- `postCreateCommand` runs and output is visible in VS Code terminal

**Current State:** UNTESTED (issue #91). Requires manual VS Code interaction.

---

## Part 4 — Test Plan

### Automation Strategy

Tests are organized into three tiers:

| Tier | Scope | Runner | CI? |
|---|---|---|---|
| **T1** — Shim unit | Individual `pelagos-docker` commands in isolation | `scripts/test-devcontainer-shim.sh` | Yes |
| **T2** — Integration | Full devcontainer CLI command sequence without VS Code | `scripts/test-devcontainer-e2e.sh` (to build) | Yes |
| **T3** — IDE | VS Code "Reopen in Container" full IDE flow | Manual (issue #91) | No |

T1 and T2 must be scriptable, non-interactive, and runnable with a single command.

---

### T1 — Shim Command Tests (`test-devcontainer-shim.sh`)

Current coverage (31 tests):

| Phase | Tests | Requirements Covered |
|---|---|---|
| Preflight | 8 | R-SH-01 |
| Image check | 1 | R-SH-01 |
| Probe run | 1 | R-SH-02 |
| Container discovery | 1 | R-SH-03 |
| Container inspect | 2 | R-SH-04, R-SH-05 |
| Exec | 3 | R-SH-06 |
| Shell server + patching | 14 | R-SH-06 |
| **Cleanup** | — | — |

**Gaps (must add):**

| TC-ID | Test case | Requirement |
|---|---|---|
| TC-T1-30 | `docker ps -q -a` returns container immediately after `docker run` | R-SH-03, R-VM-02 |
| TC-T1-31 | Label filter matches label with `/` in value (path) | R-SH-03, R-VM-03 |
| TC-T1-32 | Label filter with two filters ANDed, both must match | R-SH-03 |
| TC-T1-33 | `docker inspect` `Config.Env` is array not object | R-SH-04 |
| TC-T1-34 | `docker inspect` bind mount `Source` is host path not `/mnt/...` | R-SH-04, R-SH-05 |
| TC-T1-35 | `docker inspect` `State.Running` is boolean `true` not string | R-SH-04 |
| TC-T1-36 | `docker volume create vscode && volume ls` shows it | R-SH-08 |
| TC-T1-37 | Named volume data persists across VM restart | R-VM-05 |
| TC-T1-38 | `docker events` emits start event then doesn't immediately exit | R-SH-10 |
| TC-T1-39 | `docker build` single-stage Dockerfile + `docker run` the result | R-SH-07 |
| TC-T1-40 | `docker build` multi-stage Dockerfile | R-SH-07 |
| TC-T1-41 | VM still running 5s after exited container (no premature shutdown) | R-VM-01 |

---

### T2 — Integration Test (`test-devcontainer-e2e.sh`, to build)

Drives the actual `devcontainer` CLI (not just `pelagos-docker` directly). Requires
`devcontainer` CLI installed (`npm install -g @devcontainers/cli`).

**Test cases:**

| TC-ID | Scenario | Requirements |
|---|---|---|
| TC-T2-01 | Pre-built image: `devcontainer up` exits 0, outcome=success | R-DC-01 |
| TC-T2-02 | Pre-built image: `devcontainer exec -- uname -s` = `Linux` | R-DC-01, R-DC-04 |
| TC-T2-03 | Pre-built image: `devcontainer exec -- cat /etc/os-release` contains image distro | R-DC-04 |
| TC-T2-04 | Pre-built image: labels present after up | R-VM-03, R-SH-03 |
| TC-T2-05 | Pre-built image: second `devcontainer up` on same workspace reuses container | R-DC-01 |
| TC-T2-06 | Custom Dockerfile: `devcontainer up` builds image then starts container | R-DC-02 |
| TC-T2-07 | Custom Dockerfile: `devcontainer exec -- <tool-from-dockerfile>` works | R-DC-02, R-DC-04 |
| TC-T2-08 | `devcontainer up` with `postCreateCommand` runs and succeeds | R-DC-01 |
| TC-T2-09 | `devcontainer down` stops the container cleanly | R-DC-01 |
| TC-T2-10 | Features: `devcontainer up` with node feature, `node --version` works | R-DC-03 |

**Fixture projects** (`test/fixtures/`):

```
test/fixtures/
  dc-prebuilt/       .devcontainer/devcontainer.json  { "image": "ubuntu:22.04" }
  dc-dockerfile/     .devcontainer/devcontainer.json + Dockerfile
  dc-features/       .devcontainer/devcontainer.json  { features: { node:1 } }
  dc-postcreate/     .devcontainer/devcontainer.json  { postCreateCommand: "touch /tmp/created" }
```

---

### T3 — IDE Integration (Manual)

See issue #91. Must be performed by a human with VS Code installed. Not automatable.

---

## Part 5 — Open Issues

| # | Title | Requirement | Priority |
|---|---|---|---|
| #91 | VS Code Remote-Containers full IDE integration test | R-DC-05 | Medium |
| #92 | docker build end-to-end with features + custom Dockerfile | R-SH-07, R-DC-02, R-DC-03 | High |
| #93 | Named volume backing store persistence across VM restart | R-VM-05 | High |
| #74 | Dynamic virtiofs host-directory sharing | R-SH-05 (path translation) | Low |
| NEW | Add T1 gap test cases TC-T1-30 through TC-T1-41 | All T1 gaps | High |
| NEW | Build `test-devcontainer-e2e.sh` (T2 integration harness) | R-DC-01 through R-DC-04 | High |
