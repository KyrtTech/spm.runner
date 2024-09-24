use std::io::stdout;
use tokio::sync::mpsc;

use spm_runner::{
    cli,
    commands::{self, install::RunArguments},
    config::{AppConfig, RegistryConfig},
    optional_params, web_server,
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
            min_specs,
            verbose,
            interactive,
            port,
        } => {
            let params = optional_params::load_optional_params(&user_params);

            let mut app_config = AppConfig::new(&env, &project, dry, interactive, min_specs, verbose);

            let run_args = RunArguments {
                application: &application,
                infra: &infra,
                user_params: &params,
            };

            let writer = &mut stdout();

            if interactive == true {
                let (server_tx, mut server_rx) = mpsc::channel::<String>(2048);
                let (client_tx, client_rx) = mpsc::channel::<String>(2048);

                let web_server = match web_server::init::create(port, server_tx, client_rx) {
                    Ok(web) => web,
                    Err(e) => {
                        eprintln!("Starting web server error: {}", e);
                        std::process::exit(1);
                    }
                };

                app_config.set_channels(client_tx, &mut server_rx);

                let install = commands::install::Install::new(app_config, &registry_config, writer);
                let _ = tokio::join!(install.run(run_args), web_server);
            } else {
                let _ = commands::install::Install::new(app_config, &registry_config, writer).run(run_args).await;
            }
        }
        cli::Commands::Remove {
            strategy,
            dry,
            project,
            env,
        } => {
            let writer = &mut stdout();

            let remove =
                commands::remove::Remove::new(dry, &registry_config, &project, &env, writer);
            let _ = remove.run(strategy).await;
        }
        cli::Commands::Search { name, category } => {
            let _ = commands::search::run(&name, category, &registry_config);
        }
    }

    Ok(())
}
