use std::{env, u16};
use reqwest::Url;

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

    pub fn new(url: String, port: u16) -> Self {
        RegistryConfig {
            url,
            port
        }
    }

    pub fn get_url(&self) -> Url {
        if let Ok(mut url) = Url::parse(self.url.as_str()) {
            match url.set_port(Some(self.port)) {
                Ok(_) => url,
                Err(_) => panic!("Could not find registry endpoint")
            }
        } else {
            panic!("Could not find registry endpoint")
        }
    }
}
