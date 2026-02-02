//! Terminal pane rendering and embedded PTY management.

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::data::state::{AppState, FocusedPane};
use crate::queries::tests::current_test;

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

        let cmd = CommandBuilder::new_default_prog();
        let _child = pty_pair.slave.spawn_command(cmd)?;

        let writer = pty_pair.master.take_writer()?;

        let mut reader = pty_pair.master.try_clone_reader()?;
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let parser = vt100::Parser::new(rows, cols, 1000);

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

/// Draw the terminal pane.
pub fn draw(frame: &mut Frame, state: &AppState, terminal: &Option<EmbeddedTerminal>, area: Rect) {
    let theme = state.theme;
    let is_focused = state.focused_pane == FocusedPane::Terminal;
    let border_style = if is_focused {
        Style::default().fg(theme.accent())
    } else {
        Style::default().fg(theme.dim())
    };

    let title = if is_focused {
        " Terminal (Esc to exit, Tab to switch pane) "
    } else {
        " Terminal "
    };

    let content: Vec<Line> = if let Some(ref term) = terminal {
        let screen = term.screen();
        let mut lines = Vec::new();
        let inner_height = area.height.saturating_sub(2);
        let screen_rows = screen.size().0;

        for row in 0..inner_height.min(screen_rows) {
            let mut row_str = String::new();
            for col in 0..screen.size().1 {
                let cell = screen.cell(row, col);
                if let Some(cell) = cell {
                    row_str.push(cell.contents().chars().next().unwrap_or(' '));
                } else {
                    row_str.push(' ');
                }
            }
            let text = row_str.trim_end().to_string();
            lines.push(Line::from(text));
        }

        if lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines
    } else {
        let suggested_cmd = current_test(state)
            .and_then(|t| t.suggested_command.as_ref())
            .map(|s| format!("Suggested: {}", s))
            .unwrap_or_else(|| "(No suggested command)".to_string());

        vec![
            Line::from("Terminal not available"),
            Line::from(""),
            Line::from(suggested_cmd),
        ]
    };

    let paragraph = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    frame.render_widget(paragraph, area);

    if is_focused {
        if let Some(ref term) = terminal {
            let screen = term.screen();
            let cursor_pos = screen.cursor_position();
            let cursor_x = area.x + 1 + cursor_pos.1;
            let cursor_y = area.y + 1 + cursor_pos.0;
            if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}
