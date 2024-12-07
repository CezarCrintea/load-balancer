use std::fmt;

const ROUND_ROBIN: &str = "round_robin";
const LEAST_CONNECTIONS: &str = "least_connections";

#[derive(Clone, Copy, Debug, PartialEq)]
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
