use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use requests::{send_request, RequestType};
use std::{
    io::{self, Error},
    sync::Arc,
};

mod requests;

mod tui_utils;
use tui_utils::{cleanup_terminal, get_end_of_wrapped_text, setup_terminal};

const MAX_LOG_LINES: usize = 100;

fn main() -> Result<(), Error> {
    let client = Arc::new(
        reqwest::Client::builder()
            .pool_max_idle_per_host(50)
            .build()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
    );

    let mut terminal = setup_terminal()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut output = String::new();

    terminal.clear()?;

    let menu_items = vec![
        "1 - Change algo to round_robin",
        "2 - Change algo to least_connections",
        "3 - Send short work",
        "4 - Send long work",
        "5 - Reset worker servers",
        "6 - Server 1 increased duration",
        "7 - Server 1 increased error rate",
        "8 - Clear output",
        "q - Quit",
    ];
    let menu_item_max_len = menu_items.iter().map(|item| item.len()).max().unwrap();

    loop {
        terminal.draw(|frame| {
            let width = frame.area().width as usize;

            let mut menu_text = String::new();
            for item in &menu_items {
                let last_line_len = menu_text.lines().last().map_or(0, |line| line.len());
                if last_line_len + menu_item_max_len + 1 > width {
                    menu_text.push('\n');
                }
                menu_text.push_str(&format!(" {:width$}", item, width = menu_item_max_len));
            }

            let menu_text_height = menu_text.lines().count() as u16;
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [Constraint::Length(menu_text_height + 2), Constraint::Min(0)].as_ref(),
                )
                .split(frame.area());

            let menu = Paragraph::new(menu_text)
                .block(Block::default().borders(Borders::ALL).title("Menu"));

            let text = get_end_of_wrapped_text(&output, chunks[1]);
            let output_block = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Output"))
                .wrap(Wrap { trim: false });

            frame.render_widget(menu, chunks[0]);
            frame.render_widget(output_block, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == event::KeyEventKind::Release {
                    continue;
                }

                match key_event.code {
                    KeyCode::Char('1') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::ChangeAlgorithm {
                            new_algo: "round_robin".to_string(),
                        }
                        .build(client.clone())
                        .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));
                    }
                    KeyCode::Char('2') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::ChangeAlgorithm {
                            new_algo: "least_connections".to_string(),
                        }
                        .build(client.clone())
                        .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));
                    }
                    KeyCode::Char('3') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::Work { multiplier: 1 }
                            .build(client.clone())
                            .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));
                    }
                    KeyCode::Char('4') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::Work { multiplier: 10 }
                            .build(client.clone())
                            .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));
                    }
                    KeyCode::Char('5') => {
                        output.push_str("\nSending requests...\n");
                        let req = RequestType::SetupWorker {
                            server: 0,
                            min_duration: 10,
                            max_duration: 1000,
                            error_rate: 0.0,
                        }
                        .build(client.clone())
                        .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));

                        let req = RequestType::SetupWorker {
                            server: 1,
                            min_duration: 10,
                            max_duration: 1000,
                            error_rate: 0.0,
                        }
                        .build(client.clone())
                        .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));

                        let req = RequestType::SetupWorker {
                            server: 2,
                            min_duration: 10,
                            max_duration: 1000,
                            error_rate: 0.0,
                        }
                        .build(client.clone())
                        .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));
                    }
                    KeyCode::Char('6') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::SetupWorker {
                            server: 1,
                            min_duration: 1000,
                            max_duration: 2000,
                            error_rate: 0.0,
                        }
                        .build(client.clone())
                        .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));
                    }
                    KeyCode::Char('7') => {
                        output.push_str("\nSending request...\n");
                        let req = RequestType::SetupWorker {
                            server: 1,
                            min_duration: 500,
                            max_duration: 1000,
                            error_rate: 0.33,
                        }
                        .build(client.clone())
                        .unwrap();
                        runtime.spawn(send_request(client.clone(), req, tx.clone()));
                    }
                    KeyCode::Char('8') => {
                        output = String::new();
                    }
                    KeyCode::Char('q') => {
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
