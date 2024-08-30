use std::io::stdout;

use spm_runner::{
    cli,
    commands::{self, install::RunArguments},
    config::{AppConfig, RegistryConfig},
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
            project,
            env,
        } => {
            let params = optional_params::load_optional_params(&user_params);

            let app_config = AppConfig::new(&env, &project, dry, false);

            let run_args = RunArguments {
                application: &application,
                infra: &infra,
                user_params: &params,
            };

            let writer = &mut stdout();

            let install = commands::install::Install::new(app_config, &registry_config, writer);
            let _ = install.run(run_args).await;
        }
        cli::Commands::Remove {
            strategy,
            dry
        } => {
            let writer = &mut stdout();

            let remove = commands::remove::Remove::new(dry, &registry_config, writer);
            let _ = remove.run(strategy).await;
        }
        cli::Commands::Search { name, category } => {
            let _ = commands::search::run(&name, category, &registry_config);
        }
    }

    Ok(())
}
