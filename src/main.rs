use std::io::stdout;

use spm_runner::{
    cli,
    commands::{self, install::RunArguments},
    config::RegistryConfig,
    optional_params,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry_config = RegistryConfig::default().update_from_env();

    println!("Using registry config {:?}", registry_config);

    let command = cli::get_args();

    match command {
        cli::Commands::Install {
            application,
            infra,
            user_params,
            dry,
            project: _,
            env: _,
        } => {
            let params = optional_params::load_optional_params(&user_params);

            let run_args = RunArguments {
                application: &application,
                infra: &infra,
                dry,
                user_params: &params,
                registry_config: &registry_config,
            };

            let _ = commands::install::run(
                run_args,
                &mut stdout(),
            )
            .await;
        }
        cli::Commands::Search { name, category } => {
            let _ = commands::search::run(&name, category, &registry_config);
        }
    }

    Ok(())
}
