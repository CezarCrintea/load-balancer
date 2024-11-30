use crate::{balancing_algorithm::BalancingAlgorithm, server::Server};

#[derive(Debug)]
pub struct LoadBalancer {
    servers: Vec<Server>,
    current_server: usize,
    algorithm: BalancingAlgorithm,
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
        })
    }

    pub fn next_server(&mut self) -> &Server {
        match self.algorithm {
            BalancingAlgorithm::RoundRobin => {
                let servers_count = self.servers.len();
                let server = &mut self.servers[self.current_server];
                self.current_server = (self.current_server + 1) % servers_count;
                server.increment_connections();
                server
            }
            BalancingAlgorithm::LeastConnections => {
                let server = self
                    .servers
                    .iter_mut()
                    .min_by_key(|s| s.get_connections())
                    .unwrap();
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
}
