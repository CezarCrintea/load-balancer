use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use reqwest::StatusCode;
use std::{
    collections::HashMap,
    io::{self, Error, Stdout},
};
use tokio::task;

const MAX_LOG_LINES: usize = 100;

enum RequestType {
    ChangeAlgorithm(String),
    Work(u64, StatusCode),
}

impl RequestType {
    fn build(&self) -> Result<reqwest::Request, reqwest::Error> {
        match self {
            RequestType::ChangeAlgorithm(new_algo) => build_change_algo_request(new_algo),
            RequestType::Work(duration, status_code) => build_work_request(duration, status_code),
        }
    }
}

fn build_change_algo_request(new_algo: &str) -> Result<reqwest::Request, reqwest::Error> {
    let client = reqwest::Client::new();

    let mut data = HashMap::new();
    data.insert("algo", new_algo.to_string());

    client.post("http://127.0.0.1/algo").json(&data).build()
}

fn build_work_request(
    duration: &u64,
    status_code: &StatusCode,
) -> Result<reqwest::Request, reqwest::Error> {
    let client = reqwest::Client::new();

    let mut data = HashMap::new();
    data.insert("duration", duration.to_string());
    data.insert("status_code", status_code.as_u16().to_string());

    client.post("http://127.0.0.1/work").json(&data).build()
}

async fn send_request(req: reqwest::Request, tx: tokio::sync::mpsc::Sender<String>) {
    task::spawn(async move {
        let client = reqwest::Client::new();
        if let Ok(response) = client.execute(req).await {
            if let Ok(text) = response.text().await {
                let _ = tx.send(format!("Response received: {}", text)).await;
            } else {
                let _ = tx.send(String::from("Failed to read response text.")).await;
            }
        } else {
            let _ = tx.send(String::from("Failed to send request.")).await;
        }
    });
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, io::Error> {
    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn cleanup_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(io::stdout(), crossterm::cursor::Show)?;
    execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut terminal = setup_terminal()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut output = String::new();

    terminal.clear()?;

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(frame.area());

            let menu = Paragraph::new("1 - Change algo to round_robin. 2 - Change algo to least_connections. 3 - Process standard work. 4 - Process long work. 5 - Process work with error. 9 - Quit.")
                .block(Block::default().borders(Borders::ALL).title("Menu"));

            let text = get_end_of_wrapped_text(&output, chunks[1]);
            let output_block =
                Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Output"));

            frame.render_widget(menu, chunks[0]);
            frame.render_widget(output_block, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == event::KeyEventKind::Release {
                    continue;
                }

                match key_event.code {
                    KeyCode::Char('1') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::ChangeAlgorithm("round_robin".to_string())
                            .build()
                            .unwrap();
                        runtime.spawn(send_request(req, tx.clone()));
                    }
                    KeyCode::Char('2') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::ChangeAlgorithm("least_connections".to_string())
                            .build()
                            .unwrap();
                        runtime.spawn(send_request(req, tx.clone()));
                    }
                    KeyCode::Char('3') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::Work(10, StatusCode::OK).build().unwrap();
                        runtime.spawn(send_request(req, tx.clone()));
                    }
                    KeyCode::Char('4') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::Work(5000, StatusCode::OK).build().unwrap();
                        runtime.spawn(send_request(req, tx.clone()));
                    }
                    KeyCode::Char('5') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::Work(100, StatusCode::INTERNAL_SERVER_ERROR)
                            .build()
                            .unwrap();
                        runtime.spawn(send_request(req, tx.clone()));
                    }
                    KeyCode::Char('9') => {
                        output.push_str("\nQuitting...");
                        break;
                    }
                    _ => {}
                }
            }
        }

        while let Ok(message) = rx.try_recv() {
            output.push_str(&format!("\n{}\n", message));

            let log_lines: Vec<&str> = output.lines().collect();
            if log_lines.len() > MAX_LOG_LINES {
                output = log_lines[log_lines.len() - MAX_LOG_LINES..].join("\n");
            }
        }
    }

    cleanup_terminal()?;
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
