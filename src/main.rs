mod cli;
mod config;
mod defaults;

use config::RegistryConfig;

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
        }
        cli::Commands::Search { name, category } => {
            let _ = do_search(&name, category, &registry_config);
        }
    }

    Ok(())
}
