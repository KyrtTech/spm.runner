use std::io::Write;

use crate::config::RegistryConfig;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstallStrategyArgument {
    name: String,
    param: String,
    description: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstallStrategyResponse {
    title: String,
    description: String,
    uri: String,
    scripts: Vec<String>,
    #[serde(rename = "downloadURL")]
    download_url: Option<String>,
    runner: Option<String>,
    #[serde(rename = "optionalParams")]
    optional_params: Vec<InstallStrategyArgument>,
    alternatives: Vec<InstallStrategyResponse>,
}

impl InstallStrategyResponse {
    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn download_url(&self) -> &Option<String> {
        &self.download_url
    }

    pub fn uri(&self) -> &String {
        &self.uri
    }

    pub fn scripts(&self) -> &Vec<String> {
        &self.scripts
    }
}

pub async fn find_install_strategy<W: Write>(
    app_name: &str,
    infra: &str,
    config: &RegistryConfig,
    writer: &mut W,
) -> anyhow::Result<Vec<InstallStrategyResponse>> {
    let mut install_url = config.get_url().join("find-install-strategy")?;

    let url_with_query = install_url
        .query_pairs_mut()
        .append_pair("application", app_name)
        .append_pair("infra", infra)
        .finish();

    let _ = writeln!(
        writer,
        "Doing install for: {}, {}. Using url {}",
        app_name, infra, url_with_query
    );

    let response = reqwest::get(url_with_query.as_ref())
        .await?
        .json::<Vec<InstallStrategyResponse>>()
        .await?;

    Ok(response)
}
