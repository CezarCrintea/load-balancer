use ansi_to_tui::IntoText;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{
    io::{self, Stdout},
    path::Path,
};

use tokio::io::AsyncBufReadExt;
use tokio::process::Command as AsyncCommand;
use tokio::sync::mpsc;
use tokio::task;

const MAX_LOG_LINES: usize = 100;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    spawn_process(tx.clone(), "load-balancer".to_string(), 0, None).await;

    for (i, port) in (3000..3003).enumerate() {
        spawn_process(
            tx.clone(),
            "worker-server".to_string(),
            i + 1,
            Some(vec![("PORT".to_string(), port.to_string())]),
        )
        .await;
    }

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut logs: Vec<String> = vec![String::new(); 4];

    loop {
        if let Ok((idx, log)) = rx.try_recv() {
            logs[idx].push_str(&format!("{}\n", log));
            let log_lines: Vec<&str> = logs[idx].lines().collect();
            if log_lines.len() > MAX_LOG_LINES {
                logs[idx] = log_lines[log_lines.len() - MAX_LOG_LINES..].join("\n");
            }
        }

        if let Err(e) = draw_ui(&mut terminal, &logs) {
            eprintln!("Error drawing UI: {}", e);
            break;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == event::KeyEventKind::Release {
                    continue;
                }
                match key_event.code {
                    KeyCode::Char('c') => {
                        for i in 0..4 {
                            logs[i] = String::new();
                        }
                    }
                    KeyCode::Char('q') => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    terminal.clear()?;
    Ok(())
}

fn draw_ui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    logs: &[String],
) -> Result<(), io::Error> {
    terminal.draw(|f| {
        let size = f.area();

        let background = Block::default().style(Style::default().bg(Color::Black).fg(Color::White));
        f.render_widget(background, size);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());

        let upper_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(rows[0]);

        let lower_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(rows[1]);

        let lb_output = get_end_of_wrapped_text(&logs[0], upper_row[0]);
        let lb_block = Paragraph::new(lb_output)
            .block(
                Block::default()
                    .title("Load Balancer")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::White));
        f.render_widget(lb_block, upper_row[0]);

        let worker1_output = get_end_of_wrapped_text(&logs[1], lower_row[0]);
        let worker1_block = Paragraph::new(worker1_output)
            .block(
                Block::default()
                    .title("Worker 1")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .style(Style::default().fg(Color::White));
        f.render_widget(worker1_block, lower_row[0]);

        let worker2_output = get_end_of_wrapped_text(&logs[2], lower_row[1]);
        let worker2_block = Paragraph::new(worker2_output)
            .block(
                Block::default()
                    .title("Worker 2")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .style(Style::default().fg(Color::White));
        f.render_widget(worker2_block, lower_row[1]);

        let worker3_output = get_end_of_wrapped_text(&logs[3], lower_row[2]);
        let worker3_block = Paragraph::new(worker3_output)
            .block(
                Block::default()
                    .title("Worker 3")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .style(Style::default().fg(Color::White));
        f.render_widget(worker3_block, lower_row[2]);
    })?;

    Ok(())
}

fn get_end_of_wrapped_text(text: &str, area: Rect) -> String {
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

async fn spawn_process(
    tx: mpsc::UnboundedSender<(usize, String)>,
    name: String,
    idx: usize,
    env: Option<Vec<(String, String)>>,
) {
    let executable_file = {
        #[cfg(target_os = "windows")]
        {
            format!("{}.exe", name.clone())
        }
        #[cfg(not(target_os = "windows"))]
        {
            name.clone()
        }
    };
    let sibling_path = Path::new(".")
        .join(name.clone())
        .join("target")
        .join("debug")
        .join(executable_file.clone());
    let parent_path = Path::new("..")
        .join(name.clone())
        .join("target")
        .join("debug")
        .join(executable_file.clone());
    let executable_path = if sibling_path.exists() {
        sibling_path.to_str().unwrap().to_string()
    } else if parent_path.exists() {
        parent_path.to_str().unwrap().to_string()
    } else {
        panic!(
            "Executable not found in expected locations: {:?} or {:?}",
            sibling_path, parent_path
        );
    };

    task::spawn(async move {
        let mut cmd = AsyncCommand::new(executable_path);

        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| {
                panic!("Failed to spawn process {}: {}", name.clone(), e);
            });
        if let Some(stdout) = child.stdout.take() {
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await.unwrap_or_else(|_| None) {
                let line = line.into_text().unwrap();
                let _ = tx.send((idx, format!("{}", line)));
            }
        }
    });
}
