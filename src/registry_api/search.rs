use std::io::Write;

use crate::config::RegistryConfig;

use super::install::{InstallStrategyArgument, InstallStrategySupportFile};

static EMPTY_VEC: Vec<String> = vec![];

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

impl InstallStrategyResponse {
    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn uninstall_scripts(&self) -> &Vec<String> {
        match &self.uninstall_scripts {
            Some(paths) => &paths,
            None => &EMPTY_VEC,
        }
    }

    pub fn support_files(&self) -> &Vec<InstallStrategySupportFile> {
        &self.support_files
    }
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
