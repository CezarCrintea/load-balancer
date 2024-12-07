use crate::{balancing_algorithm::BalancingAlgorithm, server::Server};
use chrono::{DateTime, Utc};
use tracing::info;

const MIN_SECONDS_BETWEEN_ALGO_CHANGES: u64 = 5;

#[derive(Debug)]
pub struct LoadBalancer {
    servers: Vec<Server>,
    current_server: usize,
    algorithm: BalancingAlgorithm,
    last_check: DateTime<Utc>,
}

impl LoadBalancer {
    pub fn new(servers: Vec<Server>) -> Result<Self, String> {
        if servers.is_empty() {
            return Err("At least one server is required".to_string());
        }

        Ok(LoadBalancer {
            servers,
            current_server: 0,
            algorithm: BalancingAlgorithm::RoundRobin,
            last_check: Utc::now(),
        })
    }

    pub fn next_server(&mut self) -> &Server {
        self.check_conditions_and_set_best_algo();

        match self.algorithm {
            BalancingAlgorithm::RoundRobin => {
                let servers_count = self.servers.len();
                let server = &mut self.servers[self.current_server];
                self.current_server = (self.current_server + 1) % servers_count;
                server.increment_connections();
                server
            }
            BalancingAlgorithm::LeastConnections => {
                let (index, _) = self
                    .servers
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, s)| s.get_connections())
                    .unwrap();
                self.current_server = index;
                let server = &mut self.servers[self.current_server];
                server.increment_connections();
                server
            }
        }
    }

    pub fn set_algorithm(&mut self, algorithm: BalancingAlgorithm) {
        self.algorithm = algorithm;
    }

    pub fn get_server_by_address(&mut self, address: &str) -> Option<&mut Server> {
        self.servers.iter_mut().find(|s| s.get_address() == address)
    }

    fn check_conditions_and_set_best_algo(&mut self) {
        if self.servers.len() == 1 {
            return;
        }

        let mut recommended_algo = self.algorithm;

        match self.algorithm {
            BalancingAlgorithm::RoundRobin => {
                let max_connections = self
                    .servers
                    .iter()
                    .max_by_key(|s| s.get_connections())
                    .unwrap()
                    .get_connections();
                let min_connections = self
                    .servers
                    .iter()
                    .min_by_key(|s| s.get_connections())
                    .unwrap()
                    .get_connections();
                if max_connections - min_connections > 3 {
                    recommended_algo = BalancingAlgorithm::LeastConnections;
                }
            }
            BalancingAlgorithm::LeastConnections => {
                let (new_server, _) = self
                    .servers
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, s)| s.get_connections())
                    .unwrap();
                if new_server == self.current_server {
                    recommended_algo = BalancingAlgorithm::RoundRobin;
                }
            }
        }

        let now = Utc::now();
        let duration = now.signed_duration_since(self.last_check);
        if duration.num_seconds() < MIN_SECONDS_BETWEEN_ALGO_CHANGES as i64 {
            return;
        }

        if recommended_algo == self.algorithm {
            return;
        }

        self.set_algorithm(recommended_algo);

        self.last_check = now;

        info!(
            "Algorithm changed to {} due to server conditions",
            recommended_algo
        );
    }
}
