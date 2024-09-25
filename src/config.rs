use reqwest::Url;
use std::env;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::interactive_protocol;

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

pub struct Comm<'a> {
    sender: Sender<interactive_protocol::Message>,
    receiver: &'a mut Receiver<interactive_protocol::Message>,
}

impl<'a> Comm<'a> {
    pub fn sender(&self) -> &Sender<interactive_protocol::Message> {
        &self.sender
    }

    pub fn receiver(&mut self) -> &mut Receiver<interactive_protocol::Message> {
        self.receiver
    }
}

pub struct AppConfig<'a, 'b> {
    dry_run: bool,
    interactive: bool,
    env: &'a str,
    project: &'a str,
    min_specs: bool,
    verbose: bool,
    comm: Option<Comm<'b>>,
}

impl<'a, 'b> AppConfig<'a, 'b> {
    pub fn new(
        env: &'a str,
        project: &'a str,
        dry_run: bool,
        interactive: bool,
        min_specs: bool,
        verbose: bool,
    ) -> AppConfig<'a, 'b> {
        AppConfig {
            dry_run,
            interactive,
            env,
            project,
            min_specs,
            verbose,
            comm: None,
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

    pub fn use_min_specs(&self) -> bool {
        self.min_specs
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn set_channels(
        &mut self,
        sender: Sender<interactive_protocol::Message>,
        receiver: &'b mut Receiver<interactive_protocol::Message>,
    ) {
        self.comm = Some(Comm { receiver, sender });
    }

    pub fn comm(&mut self) -> Option<&mut Comm<'b>> {
        self.comm.as_mut()
    }
}
