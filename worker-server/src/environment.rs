use std::env;

pub enum Environment {
    Local,
    DockerCompose,
}

const LOCAL: &str = "local";
const DOCKER_COMPOSE: &str = "docker-compose";

impl Environment {
    pub fn from_env() -> Self {
        match env::var("APP_ENVIRONMENT") {
            Ok(environment) => match environment.as_str() {
                LOCAL => Environment::Local,
                DOCKER_COMPOSE => Environment::DockerCompose,
                _ => panic!(
                    "Invalid environment {}. Valid values are '{}' or '{}'",
                    environment, LOCAL, DOCKER_COMPOSE
                ),
            },
            Err(_) => Environment::Local,
        }
    }
}
