use std::{
    fs,
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

pub fn start_ticks(pid: u32) -> io::Result<u64> {
    let stat = fs::read(format!("/proc/{pid}/stat"))?;
    let name_end = stat.iter().rposition(|byte| *byte == b')')
        .ok_or_else(|| io::Error::other("invalid process stat"))?;
    let value = stat[name_end + 1..].split(|byte| byte.is_ascii_whitespace())
        .filter(|field| !field.is_empty()).nth(19)
        .ok_or_else(|| io::Error::other("missing process start time"))?;
    std::str::from_utf8(value).ok().and_then(|text| text.parse().ok())
        .ok_or_else(|| io::Error::other("invalid process start time"))
}

pub struct Target(OwnedFd);

impl Target {
    pub fn open(pid: u32, expected_start: u64) -> io::Result<Self> {
        if pid <= 1 || pid == std::process::id() {
            return Err(io::Error::other("Taskyy protects this process"));
        }
        // Open the pidfd before checking the process identity.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: pidfd_open returned a new descriptor.
        let target = Self(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        if start_ticks(pid)? != expected_start {
            return Err(io::Error::other("process changed since the last sample"));
        }
        Ok(target)
    }

    pub fn terminate(self) -> io::Result<()> {
        // SAFETY: the pidfd stays open for the syscall.
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                self.0.as_raw_fd(),
                libc::SIGTERM,
                std::ptr::null::<libc::siginfo_t>(),
                0u32,
            )
        };
        if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::process::ExitStatusExt, process::Command};

    #[test]
    fn signals_only_the_confirmed_child() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let start = start_ticks(child.id()).unwrap();
        assert!(Target::open(child.id(), start + 1).is_err());
        drop(Target::open(child.id(), start).unwrap());
        assert!(child.try_wait().unwrap().is_none());
        Target::open(child.id(), start).unwrap().terminate().unwrap();
        assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGTERM));
    }
}
