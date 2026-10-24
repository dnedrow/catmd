//! Active terminal probe: asks the terminal about Kitty graphics, SIXEL and
//! its cell size, then reads the replies with a timeout.
//!
//! The wire handling is separated from the TTY so it can be tested with a fake.

use std::io;
use std::time::{Duration, Instant};

/// How long to wait for the terminal to answer before assuming it will not.
pub const TIMEOUT: Duration = Duration::from_millis(250);

/// A Kitty graphics query (answered only by terminals that support it),
/// a cell-size report request, then primary device attributes, which every
/// terminal answers and which therefore marks the end of the replies.
const QUERY: &[u8] = b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[16t\x1b[c";

/// What the terminal reported.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbeResult {
    pub kitty: bool,
    pub sixel: bool,
    /// Cell `(width, height)` in pixels.
    pub cell: Option<(u32, u32)>,
}

/// A two-way channel to the terminal.
pub trait ProbeIo {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()>;
    /// Wait up to `timeout` for input; an empty result means nothing arrived.
    fn recv(&mut self, timeout: Duration) -> io::Result<Vec<u8>>;
}

/// Run the probe. `None` means the terminal never completed the exchange.
pub fn run(io: &mut dyn ProbeIo, timeout: Duration) -> Option<ProbeResult> {
    io.send(QUERY).ok()?;
    let deadline = Instant::now() + timeout;
    let mut reply = Vec::new();
    loop {
        if find_da1(&reply).is_some() {
            return Some(parse(&reply));
        }
        let remaining = deadline.checked_duration_since(Instant::now())?;
        reply.extend(io.recv(remaining).ok()?);
    }
}

/// The primary device attributes reply `ESC [ ? <params> c`, as the byte range
/// of its parameters.
fn find_da1(reply: &[u8]) -> Option<&[u8]> {
    let start = find(reply, b"\x1b[?")? + 3;
    let len = reply[start..].iter().position(|&b| b == b'c')?;
    Some(&reply[start..start + len])
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Interpret the bytes the terminal sent back.
pub fn parse(reply: &[u8]) -> ProbeResult {
    let kitty = find(reply, b"\x1b_Gi=31;OK").is_some();
    let sixel = find_da1(reply)
        .map(|params| params.split(|&b| b == b';').any(|p| p == b"4"))
        .unwrap_or(false);
    ProbeResult {
        kitty,
        sixel,
        cell: parse_cell(reply),
    }
}

/// The `ESC [ 6 ; <height> ; <width> t` cell-size report.
fn parse_cell(reply: &[u8]) -> Option<(u32, u32)> {
    let start = find(reply, b"\x1b[6;")? + 4;
    let len = reply[start..].iter().position(|&b| b == b't')?;
    let text = std::str::from_utf8(&reply[start..start + len]).ok()?;
    let (height, width) = text.split_once(';')?;
    let (height, width) = (height.parse::<u32>().ok()?, width.parse::<u32>().ok()?);
    (width > 0 && height > 0).then_some((width, height))
}

/// Probe the controlling terminal, restoring its mode afterwards.
#[cfg(unix)]
pub fn probe_terminal() -> Option<ProbeResult> {
    let mut tty = unix::Tty::open()?;
    run(&mut tty, TIMEOUT)
}

#[cfg(not(unix))]
pub fn probe_terminal() -> Option<ProbeResult> {
    None
}

#[cfg(unix)]
mod unix {
    use std::fs::{File, OpenOptions};
    use std::io::{self, Read, Write};
    use std::os::fd::AsRawFd;
    use std::time::Duration;

    use super::ProbeIo;

    /// `/dev/tty` in non-canonical, no-echo mode until dropped.
    pub struct Tty {
        file: File,
        original: libc::termios,
    }

    impl Tty {
        pub fn open() -> Option<Self> {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
                .ok()?;
            let fd = file.as_raw_fd();
            // SAFETY: `fd` is a valid open descriptor and `termios` is plain data
            // that tcgetattr fully initializes on success.
            let original = unsafe {
                let mut termios = std::mem::zeroed::<libc::termios>();
                if libc::tcgetattr(fd, &mut termios) != 0 {
                    return None;
                }
                termios
            };
            let mut raw = original;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            raw.c_cc[libc::VMIN] = 0;
            raw.c_cc[libc::VTIME] = 0;
            // SAFETY: `fd` is valid and `raw` is an initialized termios.
            if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
                return None;
            }
            Some(Self { file, original })
        }
    }

    impl Drop for Tty {
        fn drop(&mut self) {
            // SAFETY: the descriptor is still open and `original` came from tcgetattr.
            unsafe {
                libc::tcsetattr(self.file.as_raw_fd(), libc::TCSANOW, &self.original);
            }
        }
    }

    impl ProbeIo for Tty {
        fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.file.write_all(bytes)?;
            self.file.flush()
        }

        fn recv(&mut self, timeout: Duration) -> io::Result<Vec<u8>> {
            let mut poll = libc::pollfd {
                fd: self.file.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let millis = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
            // SAFETY: `poll` is a single valid pollfd.
            let ready = unsafe { libc::poll(&mut poll, 1, millis) };
            if ready < 0 {
                let err = io::Error::last_os_error();
                return if err.kind() == io::ErrorKind::Interrupted {
                    Ok(Vec::new())
                } else {
                    Err(err)
                };
            }
            if ready == 0 {
                return Ok(Vec::new());
            }
            let mut buffer = [0u8; 256];
            let n = self.file.read(&mut buffer)?;
            Ok(buffer[..n].to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        sent: Vec<u8>,
        replies: Vec<Vec<u8>>,
    }

    impl ProbeIo for Fake {
        fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.sent.extend_from_slice(bytes);
            Ok(())
        }

        fn recv(&mut self, timeout: Duration) -> io::Result<Vec<u8>> {
            if self.replies.is_empty() {
                std::thread::sleep(timeout);
                return Ok(Vec::new());
            }
            Ok(self.replies.remove(0))
        }
    }

    #[test]
    fn parses_kitty_sixel_and_cell_size() {
        let reply = b"\x1b_Gi=31;OK\x1b\\\x1b[6;34;17t\x1b[?62;4;22c";
        assert_eq!(
            parse(reply),
            ProbeResult {
                kitty: true,
                sixel: true,
                cell: Some((17, 34)),
            }
        );
    }

    #[test]
    fn da1_without_the_sixel_attribute() {
        let result = parse(b"\x1b[?62;14;22c");
        assert!(!result.sixel && !result.kitty);
        assert_eq!(result.cell, None);
    }

    #[test]
    fn replies_split_across_reads_are_joined() {
        let mut io = Fake {
            sent: Vec::new(),
            replies: vec![b"\x1b[?6".to_vec(), b"2;4c".to_vec()],
        };
        let result = run(&mut io, Duration::from_secs(1)).unwrap();
        assert!(result.sixel);
        assert!(io.sent.starts_with(b"\x1b_Gi=31"));
        assert!(io.sent.ends_with(b"\x1b[c"));
    }

    #[test]
    fn a_silent_terminal_times_out() {
        let mut io = Fake {
            sent: Vec::new(),
            replies: Vec::new(),
        };
        let started = Instant::now();
        assert_eq!(run(&mut io, Duration::from_millis(40)), None);
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
