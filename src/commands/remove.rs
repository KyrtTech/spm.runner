use std::io::Write;

use crate::{config::RegistryConfig, registry_api::{self, search::InstallStrategyResponse} };

pub struct Remove<'a, W: Write> {
    dry: bool,
    registry_config: &'a RegistryConfig,
    writer: &'a mut W,
}

impl<'a, W: Write> Remove<'a, W> {
    pub fn new(dry: bool, registry_config: &'a RegistryConfig, writer: &'a mut W) -> Self {
        Remove {
            dry,
            registry_config,
            writer,
        }
    }

    pub async fn run(self, strategy_name: String) -> anyhow::Result<()> {
        let result = registry_api::search::search_install_strategy(strategy_name, self.registry_config, self.writer).await;
        let strategy: InstallStrategyResponse;

        match result {
            Ok(str) => strategy = str,
            Err(err) => { 
                eprintln!("{:?}", err);
                return Ok(())
            }
        };

        if self.dry {
            let _ = writeln!(
                self.writer,
                "Found install strategy: {}",
                serde_json::to_string_pretty(&strategy).unwrap_or_default()
            );

            return Ok(())
        }

        Ok(())
    }
}
