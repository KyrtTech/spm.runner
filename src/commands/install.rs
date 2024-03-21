use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

use tempfile::{NamedTempFile, TempDir};

use crate::config::RegistryConfig;


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

pub async fn run(application: &str, infra: &str, registry_config: &RegistryConfig) -> anyhow::Result<()> {
    let strategies = do_install(application, infra, registry_config).await?;

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
        //  need a temp directory where to run the script
        let cwd = TempDir::new();
        let cwd_path = match cwd {
            Ok(ref c) => c.path().to_str().unwrap_or_default().to_string(),
            Err(_) => "".to_string(),
        };

        let mut permissions = file.as_file().metadata()?.permissions();
        permissions.set_mode(0o755);

        std::fs::set_permissions(file.path(), permissions).ok();

        println!("Path: {}", cwd_path);

        let mut command = Command::new(file.path())
            .current_dir(cwd_path)
            .stdout(Stdio::inherit())
            .spawn()?;

        command.wait().ok();
    }

    Ok(())
}
