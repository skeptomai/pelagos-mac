# pelagos-mac

macOS CLI for the [pelagos](https://github.com/skeptomai/pelagos) Linux container runtime.

pelagos uses Linux namespaces, cgroups, and seccomp — Linux-only kernel primitives. On
Apple Silicon a Linux VM is mandatory. This project owns that VM layer and the macOS
user experience, with a pure-Rust stack and no subsystem dependencies.

## Architecture

```
pelagos (macOS CLI)
  │
  ├── pelagos-vz       Boots a Linux VM via Apple Virtualization Framework
  │     └── objc2-virtualization (Rust bindings, auto-generated from Xcode SDK)
  │
  ├── virtiofsd        Host-side virtiofs daemon (Rust, Red Hat)
  │
  └── vsock            Forwards commands to the guest over a Unix socket
        │
        └── pelagos-guest (inside the VM)
              └── pelagos binary
```

No Go. No Lima. No gRPC daemon. See [docs/DESIGN.md](docs/DESIGN.md) for full rationale.

## Status

**Pilot phase.** The architecture is designed; implementation is in progress.

## Requirements

- macOS 13.5+ (Ventura)
- Apple Silicon (aarch64)
- Xcode command line tools

## Building

```bash
# Host binaries (macOS)
cargo build --release -p pelagos-mac

# Guest daemon (cross-compiled for Linux ARM64)
cargo build --target aarch64-unknown-linux-gnu --release -p pelagos-guest
```

## License

Apache 2.0
