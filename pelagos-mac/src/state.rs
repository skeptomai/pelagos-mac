//! Persistent VM state: PID file, Unix socket path, and mounts config
//! stored in ~/.local/share/pelagos/.

use std::io;
use std::path::PathBuf;

pub struct StateDir {
    pub pid_file: PathBuf,
    pub sock_file: PathBuf,
    pub mounts_file: PathBuf,
}

impl StateDir {
    pub fn open() -> io::Result<Self> {
        let base = base_dir()?;
        std::fs::create_dir_all(&base)?;
        Ok(Self {
            pid_file: base.join("vm.pid"),
            sock_file: base.join("vm.sock"),
            mounts_file: base.join("vm.mounts"),
        })
    }

    /// Returns the PID of the running daemon, or None if the PID file is
    /// absent, unparseable, or the process no longer exists.
    pub fn running_pid(&self) -> Option<u32> {
        let s = std::fs::read_to_string(&self.pid_file).ok()?;
        let pid: u32 = s.trim().parse().ok()?;
        let alive = unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;
        if alive { Some(pid) } else { None }
    }

    pub fn is_daemon_alive(&self) -> bool {
        self.running_pid().is_some()
    }

    /// Atomically write the PID file using rename so two racing daemons
    /// cannot both think they own the state.
    pub fn write_pid(&self, pid: u32) -> io::Result<()> {
        let tmp = self.pid_file.with_extension("pid.tmp");
        std::fs::write(&tmp, pid.to_string())?;
        std::fs::rename(&tmp, &self.pid_file)
    }

    /// Write the current daemon's mount configuration as JSON.
    pub fn write_mounts(&self, mounts: &[crate::daemon::VirtiofsShare]) -> io::Result<()> {
        let json = serde_json::to_string(mounts)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = self.mounts_file.with_extension("mounts.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.mounts_file)
    }

    /// Read the running daemon's mount configuration.  Returns an empty Vec
    /// if the file does not exist.
    pub fn read_mounts(&self) -> io::Result<Vec<crate::daemon::VirtiofsShare>> {
        match std::fs::read_to_string(&self.mounts_file) {
            Ok(s) => serde_json::from_str(&s)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Remove PID, socket, and mounts files. Best-effort; ignores errors.
    pub fn clear(&self) {
        let _ = std::fs::remove_file(&self.pid_file);
        let _ = std::fs::remove_file(&self.sock_file);
        let _ = std::fs::remove_file(&self.mounts_file);
    }
}

fn base_dir() -> io::Result<PathBuf> {
    // Respect XDG_DATA_HOME if set, otherwise ~/.local/share/pelagos
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg).join("pelagos"));
    }
    let home = std::env::var("HOME").map_err(|_| {
        io::Error::new(io::ErrorKind::NotFound, "$HOME not set")
    })?;
    Ok(PathBuf::from(home).join(".local/share/pelagos"))
}
