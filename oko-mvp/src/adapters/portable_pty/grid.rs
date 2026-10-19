//! The cell grid behind a session: the screen a shell has drawn, not the bytes
//! it wrote.
//!
//! A PTY hands over a byte stream that means something only once it is
//! interpreted — cursor moves, erases, wrapping, backspace. `alacritty_terminal`
//! is that interpreter, and this module is the seam around it: one `Screen` per
//! session, fed the session's bytes, asked to render. So `read_output` answers
//! with what a person would see rather than with the control sequences.
//!
//! Deliberately not here: scrollback navigation, selection, and colour. A
//! session's screen is read as text; anything richer belongs to whoever draws
//! the pane (ADR 0003).

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;

/// The window the grid is told to have, in cells.
///
/// `alacritty_terminal` keeps a `Dimensions` implementation of its own in
/// `pub mod test`, which is test support rather than a dependency to borrow, so
/// the three methods it asks for are implemented here.
#[derive(Clone, Copy)]
struct ScreenSize {
    columns: usize,
    screen_lines: usize,
}

impl Dimensions for ScreenSize {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

pub(super) struct Screen {
    term: Term<VoidListener>,
    parser: Processor,
    size: ScreenSize,
}

impl Screen {
    pub(super) fn new(columns: u16, screen_lines: u16) -> Self {
        let size = ScreenSize {
            columns: columns as usize,
            screen_lines: screen_lines as usize,
        };
        Self {
            term: Term::new(Config::default(), &size, VoidListener),
            parser: Processor::new(),
            size,
        }
    }

    pub(super) fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub(super) fn resize(&mut self, columns: u16, screen_lines: u16) {
        self.size = ScreenSize {
            columns: columns as usize,
            screen_lines: screen_lines as usize,
        };
        self.term.resize(self.size);
    }

    /// The visible screen as text: one line per row, trailing blanks dropped.
    ///
    /// Contract: never panics and never returns a line wider than the grid. A
    /// wide character fills two cells; the grid stores a spacer in the second,
    /// so a rendered line stays as wide as the screen.
    pub(super) fn render(&self) -> String {
        let columns = self.size.columns;
        let mut rows: Vec<Vec<char>> = (0..self.size.screen_lines)
            .map(|_| vec![' '; columns])
            .collect();
        for indexed in self.term.renderable_content().display_iter {
            let (line, column) = (indexed.point.line.0, indexed.point.column.0);
            if line < 0 || line as usize >= rows.len() || column >= columns {
                continue;
            }
            rows[line as usize][column] = indexed.cell.c;
        }
        let mut lines: Vec<String> = rows
            .into_iter()
            .map(|row| row.into_iter().collect::<String>().trim_end().to_string())
            .collect();
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// The row the cursor is on: where the shell will draw what it reads next.
    pub(super) fn cursor_line(&self) -> usize {
        let line = self.term.renderable_content().cursor.point.line.0;
        (line.max(0) as usize).min(self.size.screen_lines.saturating_sub(1))
    }
}
