use std::io::stdout;

use spm_runner::{cli, commands, config::RegistryConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry_config = RegistryConfig::default().update_from_env();

    println!("Using registry config {:?}", registry_config);

    let command = cli::get_args();

    match command {
        cli::Commands::Install {
            application,
            infra,
            dry,
        } => {
            let _ =
                commands::install::run(&application, &infra, dry, &registry_config, &mut stdout())
                    .await;
        }
        cli::Commands::Search { name, category } => {
            let _ = commands::search::run(&name, category, &registry_config);
        }
    }

    Ok(())
}
