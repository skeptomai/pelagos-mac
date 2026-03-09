//! VM lifecycle management.
//!
//! `VmConfig` describes what to boot; `Vm` owns the running VM.
//! The vsock device is exposed on the host as a Unix domain socket —
//! connect to it with `std::os::unix::net::UnixStream`.

use std::path::PathBuf;

/// Configuration for a Linux VM.
#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Path to the Linux kernel image (uncompressed or gzip).
    pub kernel: PathBuf,
    /// Path to the initial ramdisk.
    pub initrd: Option<PathBuf>,
    /// Kernel command-line arguments.
    pub cmdline: String,
    /// Root disk image (raw or qcow2).
    pub disk: PathBuf,
    /// Number of vCPUs.
    pub cpus: usize,
    /// Memory in MiB.
    pub memory_mib: usize,
    /// Unix socket path for the vsock device (host side).
    /// Guest connects via AF_VSOCK to the configured port.
    pub vsock_socket: PathBuf,
    /// vsock port the guest daemon listens on.
    pub vsock_port: u32,
    /// Directories to share via virtiofs: (host_path, mount_tag).
    pub virtiofs_shares: Vec<(PathBuf, String)>,
    /// Enable Rosetta for x86_64 Linux binaries.
    pub rosetta: bool,
}

impl VmConfig {
    pub fn builder() -> VmConfigBuilder {
        VmConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct VmConfigBuilder {
    kernel: Option<PathBuf>,
    initrd: Option<PathBuf>,
    cmdline: Option<String>,
    disk: Option<PathBuf>,
    cpus: Option<usize>,
    memory_mib: Option<usize>,
    vsock_socket: Option<PathBuf>,
    vsock_port: Option<u32>,
    virtiofs_shares: Vec<(PathBuf, String)>,
    rosetta: bool,
}

impl VmConfigBuilder {
    pub fn kernel(mut self, p: impl Into<PathBuf>) -> Self { self.kernel = Some(p.into()); self }
    pub fn initrd(mut self, p: impl Into<PathBuf>) -> Self { self.initrd = Some(p.into()); self }
    pub fn cmdline(mut self, s: impl Into<String>) -> Self { self.cmdline = Some(s.into()); self }
    pub fn disk(mut self, p: impl Into<PathBuf>) -> Self { self.disk = Some(p.into()); self }
    pub fn cpus(mut self, n: usize) -> Self { self.cpus = Some(n); self }
    pub fn memory_mib(mut self, n: usize) -> Self { self.memory_mib = Some(n); self }
    pub fn vsock_socket(mut self, p: impl Into<PathBuf>) -> Self { self.vsock_socket = Some(p.into()); self }
    pub fn vsock_port(mut self, p: u32) -> Self { self.vsock_port = Some(p); self }
    pub fn virtiofs(mut self, host: impl Into<PathBuf>, tag: impl Into<String>) -> Self {
        self.virtiofs_shares.push((host.into(), tag.into())); self
    }
    pub fn rosetta(mut self, enabled: bool) -> Self { self.rosetta = enabled; self }

    pub fn build(self) -> Result<VmConfig, &'static str> {
        Ok(VmConfig {
            kernel: self.kernel.ok_or("kernel required")?,
            initrd: self.initrd,
            cmdline: self.cmdline.unwrap_or_else(|| "console=hvc0".into()),
            disk: self.disk.ok_or("disk required")?,
            cpus: self.cpus.unwrap_or(2),
            memory_mib: self.memory_mib.unwrap_or(1024),
            vsock_socket: self.vsock_socket.ok_or("vsock_socket required")?,
            vsock_port: self.vsock_port.unwrap_or(1024),
            virtiofs_shares: self.virtiofs_shares,
            rosetta: self.rosetta,
        })
    }
}

/// A running VM. Drop to stop it.
pub struct Vm {
    // TODO: hold the VZVirtualMachine reference from objc2-virtualization
    config: VmConfig,
}

impl Vm {
    /// Boot a VM from the given config.
    pub fn start(_config: VmConfig) -> Result<Self, crate::Error> {
        todo!("implement via objc2-virtualization VZVirtualMachine")
    }

    /// Block until the VM stops, returning its exit reason.
    pub fn wait(self) -> Result<VmExitReason, crate::Error> {
        todo!()
    }

    /// Request a clean shutdown (ACPI power-off signal).
    pub fn stop(&self) -> Result<(), crate::Error> {
        todo!()
    }

    pub fn config(&self) -> &VmConfig {
        &self.config
    }
}

#[derive(Debug)]
pub enum VmExitReason {
    Stopped,
    Error(String),
}
