//! Embedded terminal with PTY support.

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Manages an embedded terminal with PTY.
pub struct EmbeddedTerminal {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    parser: vt100::Parser,
    output_rx: Receiver<Vec<u8>>,
}

impl EmbeddedTerminal {
    /// Create a new embedded terminal with the given size.
    pub fn new(rows: u16, cols: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();

        let pty_pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Spawn a shell
        let cmd = CommandBuilder::new_default_prog();
        let _child = pty_pair.slave.spawn_command(cmd)?;

        // Get writer for sending input to PTY
        let writer = pty_pair.master.take_writer()?;

        // Set up reader in a separate thread
        let mut reader = pty_pair.master.try_clone_reader()?;
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let parser = vt100::Parser::new(rows, cols, 1000); // 1000 lines scrollback

        Ok(Self {
            master: pty_pair.master,
            writer,
            parser,
            output_rx: rx,
        })
    }

    /// Resize the terminal.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.parser.set_size(rows, cols);
    }

    /// Process any pending output from the PTY.
    pub fn poll_output(&mut self) {
        while let Ok(data) = self.output_rx.try_recv() {
            self.parser.process(&data);
        }
    }

    /// Send a character to the PTY.
    pub fn send_char(&mut self, c: char) {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        let _ = self.writer.write_all(s.as_bytes());
        let _ = self.writer.flush();
    }

    /// Send a string to the PTY.
    pub fn send_str(&mut self, s: &str) {
        let _ = self.writer.write_all(s.as_bytes());
        let _ = self.writer.flush();
    }

    /// Send a special key sequence to the PTY.
    pub fn send_key(&mut self, key: &[u8]) {
        let _ = self.writer.write_all(key);
        let _ = self.writer.flush();
    }

    /// Get the current screen contents.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }
}
