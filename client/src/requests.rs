use std::{collections::HashMap, sync::Arc};

use reqwest::StatusCode;
use tokio::task;

pub enum RequestType {
    ChangeAlgorithm(String),
    Work(u64, StatusCode),
}

impl RequestType {
    pub fn build(&self, client: Arc<reqwest::Client>) -> Result<reqwest::Request, reqwest::Error> {
        match self {
            RequestType::ChangeAlgorithm(new_algo) => build_change_algo_request(client, new_algo),
            RequestType::Work(duration, status_code) => {
                build_work_request(client, duration, status_code)
            }
        }
    }
}

fn build_change_algo_request(
    client: Arc<reqwest::Client>,
    new_algo: &str,
) -> Result<reqwest::Request, reqwest::Error> {
    let mut data = HashMap::new();
    data.insert("algo", new_algo.to_string());

    client.post("http://127.0.0.1/algo").json(&data).build()
}

fn build_work_request(
    client: Arc<reqwest::Client>,
    duration: &u64,
    status_code: &StatusCode,
) -> Result<reqwest::Request, reqwest::Error> {
    let mut data = HashMap::new();
    data.insert("duration", duration.to_string());
    data.insert("status_code", status_code.as_u16().to_string());

    client.post("http://127.0.0.1/work").json(&data).build()
}

pub async fn send_request(
    client: Arc<reqwest::Client>,
    req: reqwest::Request,
    tx: tokio::sync::mpsc::Sender<String>,
) {
    task::spawn(async move {
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
