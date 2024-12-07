use std::{collections::HashMap, sync::Arc};

use tokio::task;

pub enum RequestType {
    ChangeAlgorithm {
        new_algo: String,
    },
    Work {
        multiplier: u64,
    },
    SetupWorker {
        server: u64,
        min_duration: u64,
        max_duration: u64,
        error_rate: f64,
    },
}

impl RequestType {
    pub fn build(&self, client: Arc<reqwest::Client>) -> Result<reqwest::Request, reqwest::Error> {
        match self {
            RequestType::ChangeAlgorithm { new_algo } => {
                build_change_algo_request(client, new_algo)
            }
            RequestType::Work { multiplier } => build_work_request(client, multiplier),
            RequestType::SetupWorker {
                server,
                min_duration,
                max_duration,
                error_rate,
            } => build_setup_worker_request(client, server, min_duration, max_duration, error_rate),
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
    multiplier: &u64,
) -> Result<reqwest::Request, reqwest::Error> {
    let mut data = HashMap::new();
    data.insert("multiplier", multiplier.to_string());

    client.post("http://127.0.0.1/work").json(&data).build()
}

fn build_setup_worker_request(
    client: Arc<reqwest::Client>,
    server: &u64,
    min_duration: &u64,
    max_duration: &u64,
    error_rate: &f64,
) -> Result<reqwest::Request, reqwest::Error> {
    let mut data = HashMap::new();
    data.insert("min_duration", min_duration.to_string());
    data.insert("max_duration", max_duration.to_string());
    data.insert("error_rate", error_rate.to_string());

    let url = format!("http://127.0.0.1:{}/setup", server + 3000);
    client.post(url).json(&data).build()
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
