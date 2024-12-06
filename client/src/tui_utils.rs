use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};

use std::io::{self, Stdout};

pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, io::Error> {
    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn cleanup_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(io::stdout(), crossterm::cursor::Show)?;
    execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;
    Ok(())
}

pub fn get_end_of_wrapped_text(text: &str, area: Rect) -> String {
    let mut wrapped_lines = Vec::new();

    let height = area.height as usize - 2;
    let width = area.width as usize - 2;

    for line in text.lines() {
        let mut current_line = String::new();

        for word in line.split_whitespace() {
            if current_line.len() + word.len() + 1 > width {
                wrapped_lines.push(current_line);
                current_line = String::new();
            }

            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }

        if !current_line.is_empty() {
            wrapped_lines.push(current_line);
        }
    }

    let start = if wrapped_lines.len() > height {
        wrapped_lines.len() - height
    } else {
        0
    };

    wrapped_lines[start..].join("\n")
}
