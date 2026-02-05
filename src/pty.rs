use anyhow::Result;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::pty::openpty;
use nix::unistd::{close, dup2, execvp, fork, read, setsid, write as nix_write, ForkResult, Pid};
use std::ffi::CString;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

pub struct Pane {
    parser: vt100::Parser,
    master: OwnedFd,
    rx: Receiver<Vec<u8>>,
    #[allow(dead_code)]
    pub child_pid: Pid,
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
                drop(pty.slave);

                let flags = fcntl(master_raw, FcntlArg::F_GETFL)?;
                let mut new_flags = OFlag::from_bits_truncate(flags);
                new_flags.insert(OFlag::O_NONBLOCK);
                fcntl(master_raw, FcntlArg::F_SETFL(new_flags))?;

                let (tx, rx) = mpsc::channel();

                thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    loop {
                        match read(master_raw, &mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                if tx.send(buf[..n].to_vec()).is_err() {
                                    break;
                                }
                            }
                            Err(nix::errno::Errno::EAGAIN) => {
                                thread::sleep(std::time::Duration::from_millis(50));
                            }
                            Err(_) => break,
                        }
                    }
                });

                Ok(Self {
                    parser: vt100::Parser::new(rows, cols, 0),
                    master: pty.master,
                    rx,
                    child_pid: child,
                })
            }
        }
    }

    pub fn read_available(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(data) => self.parser.process(&data),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    pub fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }

    pub fn cursor_position(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
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
        let start = cursor_row.saturating_sub(5);
        let end = (cursor_row + 1).min(lines.len());
        lines[start..end].join("\n")
    }

    pub fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        let _ = nix_write(&self.master, data);
        Ok(())
    }
}
