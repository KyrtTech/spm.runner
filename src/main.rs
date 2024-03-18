mod cli;
mod config;
mod defaults;

use std::{io::Write, os::unix::fs::PermissionsExt, process::Command};

use config::RegistryConfig;
use tempfile::NamedTempFile;

fn do_search(
    name: &str,
    category: cli::SearchCategory,
    config: &RegistryConfig,
) -> Result<(), url::ParseError> {
    let mut search_url = config.get_url().join("search")?;

    let url_with_query = search_url
        .query_pairs_mut()
        .append_pair("name", name)
        .append_pair("type", &category.to_string())
        .finish();

    println!(
        "Doing search for: {}, {}. Using url {}",
        name, category, url_with_query
    );

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct InstallStrategyResponse {
    title: String,
    description: String,
    scripts: Vec<String>,
}

async fn do_install(
    app_name: &str,
    infra: &str,
    config: &RegistryConfig,
) -> anyhow::Result<Vec<InstallStrategyResponse>> {
    let mut install_url = config.get_url().join("find-install-strategy")?;

    let url_with_query = install_url
        .query_pairs_mut()
        .append_pair("application", app_name)
        .append_pair("infra", infra)
        .finish();

    println!(
        "Doing install for: {}, {}. Using url {}",
        app_name, infra, url_with_query
    );

    let response = reqwest::get(url_with_query.as_ref())
        .await?
        .json::<Vec<InstallStrategyResponse>>()
        .await?;

    Ok(response)
}

async fn download_scripts(scripts: &Vec<String>) -> anyhow::Result<Vec<Option<NamedTempFile>>> {
    let scripts_futures = scripts.iter().map(|url| async move {
        let response = reqwest::get(url).await.unwrap();

        if response.status().is_success() {
            if let Ok(mut file) = tempfile::NamedTempFile::new() {
                let bytes = response.bytes().await.unwrap_or_default();

                file.write_all(&bytes).ok();

                return Some(file);
            }
        }

        None
    });

    let script_paths = futures::future::join_all(scripts_futures).await;

    Ok(script_paths)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry_config = RegistryConfig::default().update_from_env();

    println!("Using registry config {:?}", registry_config);

    let command = cli::get_args();

    match command {
        cli::Commands::Install { application, infra } => {
            let strategies = do_install(&application, &infra, &registry_config).await?;

            if strategies.len() == 0 {
                println!("No valid install strategies found");
                std::process::exit(0);
            }

            let strategy = &strategies[0];

            println!("Using strategy '{}'", strategy.title);

            let paths = download_scripts(&strategy.scripts)
                .await
                .unwrap_or_default();

            let filter_paths: Vec<NamedTempFile> = paths.into_iter().filter_map(|p| p).collect();

            for file in filter_paths {
                let mut permissions = file.as_file().metadata()?.permissions();
                permissions.set_mode(0o755);

                std::fs::set_permissions(file.path(), permissions).ok();

                let output = Command::new(file.path()).output()?;

                print!("Script output: {}", String::from_utf8_lossy(&output.stdout));
            }
        }
        cli::Commands::Search { name, category } => {
            let _ = do_search(&name, category, &registry_config);
        }
    }

    Ok(())
}
