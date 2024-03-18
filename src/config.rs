use std::{env, u16};

#[derive(Debug)]
pub struct RegistryConfig {
    url: String,
    port: u16,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        RegistryConfig {
            url: super::defaults::REGISTRY.to_string(),
            port: super::defaults::REGISTRY_PORT,
        }
    }
}

impl RegistryConfig {
    pub fn update_from_env(mut self) -> Self {
        if let Ok(url) = env::var(super::defaults::REGISTRY_ENV_VAR) {
            self.url = url
        }

        if let Ok(port_str) = env::var(super::defaults::REGISTRY_PORT_ENV_VAR) {
            if let Ok(port) = port_str.parse::<u16>() {
                self.port = port;
            } else {
                eprintln!(
                    "Warning: Unable to parse {} as u16. Using default value.",
                    super::defaults::REGISTRY_PORT_ENV_VAR
                );
            }
        }

        self
    }
}
