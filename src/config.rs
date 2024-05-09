use reqwest::Url;
use std::env;

#[derive(Debug)]
pub struct RegistryConfig {
    url: String,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        RegistryConfig {
            url: super::defaults::REGISTRY.to_string(),
        }
    }
}

impl RegistryConfig {
    pub fn update_from_env(mut self) -> Self {
        if let Ok(url) = env::var(super::defaults::REGISTRY_ENV_VAR) {
            self.url = url
        }

        self
    }

    pub fn new(url: String) -> Self {
        RegistryConfig { url }
    }

    pub fn get_url(&self) -> Url {
        if let Ok(url) = Url::parse(self.url.as_str()) {
            url
        } else {
            panic!("Could not find registry endpoint")
        }
    }
}

pub struct AppConfig<'a> {
    dry_run: bool,
    interactive: bool,
    env: &'a str,
    project: &'a str,
}

impl AppConfig<'_> {
    pub fn new<'a>(
        env: &'a str,
        project: &'a str,
        dry_run: bool,
        interactive: bool,
    ) -> AppConfig<'a> {
        AppConfig {
            dry_run,
            interactive,
            env,
            project,
        }
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn interactive(&self) -> bool {
        self.interactive
    }

    pub fn env(&self) -> &str {
        self.env
    }

    pub fn project(&self) -> &str {
        self.project
    }
}
