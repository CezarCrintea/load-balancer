use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{io, path::Path};

use tokio::io::AsyncBufReadExt;
use tokio::process::Command as AsyncCommand;
use tokio::sync::mpsc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    spawn_process(tx.clone(), "load-balancer".to_string(), 0, None).await;

    // Spawn worker-server instances with different ports
    for (i, port) in (3000..3003).enumerate() {
        spawn_process(
            tx.clone(),
            "worker-server".to_string(),
            i + 1,
            Some(vec![("PORT".to_string(), port.to_string())]),
        )
        .await;
    }

    // Initialize terminal
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Process logs from subprocesses
    let mut logs: Vec<String> = vec![String::new(); 4];

    loop {
        // Listen for logs
        if let Ok((idx, log)) = rx.try_recv() {
            logs[idx].push_str(&format!("{}\n", log));
        }

        // Draw UI
        terminal.draw(|f| {
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

            let lb_output = logs[0].clone();
            let lb_block = Paragraph::new(lb_output)
                .block(
                    Block::default()
                        .title("Load Balancer")
                        .borders(Borders::ALL),
                )
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(lb_block, upper_row[0]);

            let worker1_output = logs[1].clone();
            let worker1_block = Paragraph::new(worker1_output)
                .block(Block::default().title("Worker 1").borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(Color::Green));
            f.render_widget(worker1_block, lower_row[0]);

            let worker2_output = logs[2].clone();
            let worker2_block = Paragraph::new(worker2_output)
                .block(Block::default().title("Worker 2").borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(Color::Green));
            f.render_widget(worker2_block, lower_row[1]);

            let worker3_output = logs[3].clone();
            let worker3_block = Paragraph::new(worker3_output)
                .block(Block::default().title("Worker 3").borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(Color::Green));
            f.render_widget(worker3_block, lower_row[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    terminal.clear()?;
    Ok(())
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
                let _ = tx.send((idx, format!("{}", line)));
            }
        }
    });
}
