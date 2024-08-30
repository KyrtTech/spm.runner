use std::io::Write;

use crate::config::RegistryConfig;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstallStrategyArgument {
    name: String,
    param: String,
    description: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InstallStrategySupportFile {
    #[serde(rename = "accessUrl")]
    access_url: String,
    #[serde(rename = "localPath")]
    local_path: String,
}

impl InstallStrategySupportFile {
    pub fn access_url(&self) -> &String {
        &self.access_url
    }

    pub fn local_path(&self) -> &String {
        &self.local_path
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PlanInstallStrategyResponse {
    title: String,
    description: String,
    uri: String,
    scripts: Vec<String>,
    #[serde(rename = "supportFiles")]
    support_files: Vec<InstallStrategySupportFile>,
    #[serde(rename = "downloadURL")]
    download_url: Option<String>,
    runner: Option<String>,
    #[serde(rename = "optionalParams")]
    optional_params: Vec<InstallStrategyArgument>,
    alternatives: Vec<PlanInstallStrategyResponse>,
}

impl PlanInstallStrategyResponse {
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

    pub fn support_files(&self) -> &Vec<InstallStrategySupportFile> {
        &self.support_files
    }
}

pub async fn find_install_strategy<W: Write>(
    app_name: &str,
    infra: &str,
    config: &RegistryConfig,
    writer: &mut W,
) -> anyhow::Result<Vec<PlanInstallStrategyResponse>> {
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
        .json::<Vec<PlanInstallStrategyResponse>>()
        .await?;

    Ok(response)
}
