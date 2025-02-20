use ansi_to_tui::IntoText;
use crossterm::event::{self, Event, KeyCode};
use environment::Environment;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{
    io::{self, Stdout},
    path::Path,
};
use tui_utils::{cleanup_terminal, get_end_of_wrapped_text, setup_terminal};

use tokio::process::Command as AsyncCommand;
use tokio::sync::mpsc;
use tokio::task;
use tokio::io::AsyncBufReadExt;

const MAX_LOG_LINES: usize = 100;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    launch_load_balancer(tx.clone()).await;

    let mut terminal = setup_terminal()?;
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

    cleanup_terminal()?;
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

async fn launch_load_balancer(tx: mpsc::UnboundedSender<(usize, String)>) {
    let env = Environment::from_env();
    match env {
        Environment::Local => launch_load_balancer_local(tx).await,
        Environment::DockerCompose => launch_load_balancer_docker_compose(tx).await,
    }
}

async fn launch_load_balancer_local(tx: mpsc::UnboundedSender<(usize, String)>) {
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
}

async fn launch_load_balancer_docker_compose(tx: mpsc::UnboundedSender<(usize, String)>) {
    let output = AsyncCommand::new("docker-compose")
        .arg("up")
        .arg("-d")
        .output()
        .await
        .expect("Failed to execute docker-compose");

    if !output.status.success() {
        eprintln!(
            "Error launching Docker Compose: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    println!("Docker Compose launched successfully!");

    tokio::spawn(async move {
        let containers = vec![
            "load-balancer",
            "worker-server1",
            "worker-server2",
            "worker-server3",
        ];
        let mut tasks = Vec::new();

        for (idx, container) in containers.iter().enumerate() {
            let tx = tx.clone();
            let container_name = container.to_string();

            let task = tokio::spawn(async move {
                let mut cmd = AsyncCommand::new("docker");
                cmd.arg("logs")
                    .arg("-f") // Follow logs
                    .arg(&container_name);

                let mut child = cmd
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .expect("Failed to spawn docker logs command");

                if let Some(stdout) = child.stdout.take() {
                    let reader = tokio::io::BufReader::new(stdout);
                    let mut lines = reader.lines();

                    while let Some(line) = lines.next_line().await.unwrap_or_else(|_| None) {
                        let line = line.into_text().unwrap();
                        let _ = tx.send((idx, format!("{}", line)));
                    }
                }

                child.wait().await.expect("Failed to wait for docker logs");
            });

            tasks.push(task);
        }

        tasks.into_iter().for_each(|t| {
            tokio::spawn(async move {
                t.await.expect("Failed to wait for task");
            });
        });
    });
}
