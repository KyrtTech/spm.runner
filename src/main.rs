mod cli;
mod commands;
mod config;
mod defaults;

use config::RegistryConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry_config = RegistryConfig::default().update_from_env();

    println!("Using registry config {:?}", registry_config);

    let command = cli::get_args();

    match command {
        cli::Commands::Install { application, infra } => {
            let _ = commands::install::run(&application, &infra, &registry_config).await;
        }
        cli::Commands::Search { name, category } => {
            let _ = commands::search::run(&name, category, &registry_config);
        }
    }

    Ok(())
}
