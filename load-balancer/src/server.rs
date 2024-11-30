use std::net::SocketAddr;

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