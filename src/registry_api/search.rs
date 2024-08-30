use std::io::Write;

use crate::config::RegistryConfig;

use super::install::{InstallStrategyArgument, InstallStrategySupportFile};


#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstallStrategyResponse {
    title: String,
    description: String,
    uri: String,
    scripts: Vec<String>,
    #[serde(rename="uninstallScripts")]
    uninstall_scripts: Option<Vec<String>>,
    #[serde(rename = "supportFiles")]
    support_files: Vec<InstallStrategySupportFile>,
    #[serde(rename = "downloadURL")]
    download_url: Option<String>,
    runner: Option<String>,
    #[serde(rename = "optionalParams")]
    optional_params: Vec<InstallStrategyArgument>,
}

pub async fn search_install_strategy<W: Write>(
    name: String,
    config: &RegistryConfig,
    w: &mut W,
) -> anyhow::Result<InstallStrategyResponse> {
    let mut search_url = config.get_url().join("search")?;

    let url_with_query = search_url
        .query_pairs_mut()
        .append_pair("name", &name)
        .append_pair("type", "install-strategy")
        .finish();

    let _ = writeln!(
        w,
        "Doing search for: {}, {}. Using url {}",
        name, "install-strategy", url_with_query
    );

    let response = reqwest::get(url_with_query.as_ref())
        .await?
        .json::<InstallStrategyResponse>()
        .await?;

    Ok(response)
}
