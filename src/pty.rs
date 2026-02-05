use anyhow::Result;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::pty::openpty;
use nix::sys::termios::{self, SetArg};
use nix::unistd::{close, dup2, execvp, fork, read, setsid, write as nix_write, ForkResult, Pid};
use std::ffi::CString;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

const PTY_READ_BUF_SIZE: usize = 16384;
const PTY_IDLE_SLEEP_MS: u64 = 5;

pub struct Pane {
    parser: vt100::Parser,
    master: OwnedFd,
    rx: Receiver<Vec<u8>>,
    pub child_pid: Pid,
    dsr_tail: Vec<u8>,
}

impl Pane {
    pub fn spawn_shell(rows: u16, cols: u16) -> Result<Self> {
        let pty = openpty(None, None)?;
        let master_raw = pty.master.as_raw_fd();
        let slave_raw = pty.slave.as_raw_fd();

        // Set terminal size on the slave
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe { libc::ioctl(slave_raw, libc::TIOCSWINSZ, &ws) };

        match unsafe { fork() }? {
            ForkResult::Child => {
                drop(pty.master);
                let _ = setsid();
                unsafe { libc::ioctl(slave_raw, libc::TIOCSCTTY as _, 0) };

                let _ = dup2(slave_raw, 0);
                let _ = dup2(slave_raw, 1);
                let _ = dup2(slave_raw, 2);
                if slave_raw > 2 {
                    let _ = close(slave_raw);
                }

                let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
                let shell_c = CString::new(shell.as_str()).unwrap();
                let term = CString::new("TERM=xterm-256color").unwrap();
                unsafe { libc::putenv(term.into_raw()) };

                let _ = execvp(&shell_c, &[&shell_c]);
                std::process::exit(1);
            }
            ForkResult::Parent { child } => {
                // Disable echo on slave before dropping so DSR responses aren't echoed
                if let Ok(mut attrs) = termios::tcgetattr(&pty.slave) {
                    attrs.local_flags.remove(termios::LocalFlags::ECHO);
                    let _ = termios::tcsetattr(&pty.slave, SetArg::TCSANOW, &attrs);
                }
                drop(pty.slave);

                let flags = fcntl(master_raw, FcntlArg::F_GETFL)?;
                let mut new_flags = OFlag::from_bits_truncate(flags);
                new_flags.insert(OFlag::O_NONBLOCK);
                fcntl(master_raw, FcntlArg::F_SETFL(new_flags))?;

                let (tx, rx) = mpsc::channel();

                thread::spawn(move || {
                    let mut buf = [0u8; PTY_READ_BUF_SIZE];
                    loop {
                        match read(master_raw, &mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                if tx.send(buf[..n].to_vec()).is_err() {
                                    break;
                                }
                            }
                            Err(nix::errno::Errno::EAGAIN) => {
                                thread::sleep(std::time::Duration::from_millis(PTY_IDLE_SLEEP_MS));
                            }
                            Err(_) => break,
                        }
                    }
                });

                Ok(Self {
                    parser: vt100::Parser::new(rows, cols, 1000),
                    master: pty.master,
                    rx,
                    child_pid: child,
                    dsr_tail: Vec::new(),
                })
            }
        }
    }

    pub fn read_available(&mut self) {
        let mut dsr_count = 0u32;
        loop {
            match self.rx.try_recv() {
                Ok(data) => {
                    // Check for DSR across chunk boundary
                    let mut check = std::mem::take(&mut self.dsr_tail);
                    check.extend_from_slice(&data);
                    for window in check.windows(4) {
                        if window == b"\x1b[6n" {
                            dsr_count += 1;
                        }
                    }
                    let tail_start = check.len().saturating_sub(3);
                    self.dsr_tail = check[tail_start..].to_vec();

                    self.parser.process(&data);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        for _ in 0..dsr_count {
            let (row, col) = self.parser.screen().cursor_position();
            let resp = format!("\x1b[{};{}R", row + 1, col + 1);
            let _ = self.write_bytes(resp.as_bytes());
        }

        // Reap zombie child
        use nix::sys::wait::{waitpid, WaitPidFlag};
        let _ = waitpid(self.child_pid, Some(WaitPidFlag::WNOHANG));
    }

    pub fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }

    pub fn scrollback_len(&self) -> usize {
        self.parser.screen().scrollback()
    }

    pub fn contents_with_scrollback(&self) -> String {
        self.parser.screen().contents()
    }

    pub fn cursor_position(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    pub fn hide_cursor(&self) -> bool {
        self.parser.screen().hide_cursor()
    }

    pub fn mouse_protocol_mode(&self) -> vt100::MouseProtocolMode {
        self.parser.screen().mouse_protocol_mode()
    }

    pub fn mouse_protocol_encoding(&self) -> vt100::MouseProtocolEncoding {
        self.parser.screen().mouse_protocol_encoding()
    }

    pub fn cell(&self, row: u16, col: u16) -> Option<&vt100::Cell> {
        self.parser.screen().cell(row, col)
    }

    pub fn cursor_line(&self) -> String {
        let screen = self.parser.screen();
        let cursor_row = screen.cursor_position().0 as usize;
        let contents = screen.contents();
        contents.lines().nth(cursor_row).unwrap_or("").to_string()
    }

    pub fn lines_near_cursor(&self) -> String {
        let screen = self.parser.screen();
        let cursor_row = screen.cursor_position().0 as usize;
        let contents = screen.contents();
        let lines: Vec<&str> = contents.lines().collect();
        if lines.is_empty() {
            return String::new();
        }

        // `cursor_row` is based on terminal rows; `contents.lines()` omits some trailing empty rows.
        // Clamp to the available slice bounds to avoid out-of-range panics.
        let clamped_row = cursor_row.min(lines.len().saturating_sub(1));
        let start = clamped_row.saturating_sub(5);
        let end = clamped_row + 1;
        lines[start..end].join("\n")
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &ws) };
        self.parser.set_size(rows, cols);
    }

    pub fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        let _ = nix_write(&self.master, data);
        Ok(())
    }

    pub fn terminate(&self) {
        unsafe {
            libc::kill(self.child_pid.as_raw(), libc::SIGHUP);
        }
    }
}
