use std::{fmt, net::SocketAddr};

#[derive(Debug)]
pub struct Server {
    address: String,
    connections: usize,
}

impl Server {
    pub fn new(address: String) -> Result<Self, String> {
        if address.is_empty() {
            return Err("Address cannot be empty".to_string());
        }

        if address.parse::<SocketAddr>().is_ok() {
            Ok(Server {
                address,
                connections: 0,
            })
        } else {
            Err(format!("Invalid address: {}", address))
        }
    }

    pub fn get_address(&self) -> &str {
        &self.address
    }

    pub fn get_connections(&self) -> usize {
        self.connections
    }

    pub fn increment_connections(&mut self) {
        self.connections += 1;
    }

    pub fn decrement_connections(&mut self) {
        if self.connections > 0 {
            self.connections -= 1;
        }
    }
}

const ROUND_ROBIN: &str = "round_robin";
const LEAST_CONNECTIONS: &str = "least_connections";

#[derive(Clone, Copy, Debug)]
pub enum BalancingAlgorithm {
    RoundRobin,
    LeastConnections,
}

pub struct ConversionError;

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Invalid balancing algorithm")
    }
}

impl TryFrom<&str> for BalancingAlgorithm {
    type Error = ConversionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            ROUND_ROBIN => Ok(BalancingAlgorithm::RoundRobin),
            LEAST_CONNECTIONS => Ok(BalancingAlgorithm::LeastConnections),
            _ => Err(ConversionError),
        }
    }
}

impl fmt::Display for BalancingAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            BalancingAlgorithm::RoundRobin => &ROUND_ROBIN,
            BalancingAlgorithm::LeastConnections => &LEAST_CONNECTIONS,
        };
        write!(f, "{}", name)
    }
}

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
