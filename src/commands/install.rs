use std::{
    collections::HashMap,
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

use tempfile::{NamedTempFile, TempDir};

use crate::config::RegistryConfig;
use crate::defaults;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct InstallStrategyResponse {
    title: String,
    description: String,
    scripts: Vec<String>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct InstallStrategyOutput {
    output_params: HashMap<String, String>,
}

impl Default for InstallStrategyOutput {
    fn default() -> Self {
        InstallStrategyOutput {
            output_params: HashMap::new(),
        }
    }
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

fn consolidate_script_output<'a>(
    data: InstallStrategyOutput,
    consolidate: &mut Vec<InstallStrategyOutput>,
    script_folder: &str,
) {
    //  we need to save files to persistent locations
    if let Some(ssh_key_file) = data.output_params.get("sshKey") {
        let _ = std::fs::copy(
            format!("{}/{}", script_folder, ssh_key_file),
            format!("{}.pem", consolidate.len()),
        );
    }

    consolidate.push(data);
}

pub async fn run(
    application: &str,
    infra: &str,
    registry_config: &RegistryConfig,
) -> anyhow::Result<()> {
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

    let output_params_files: Vec<Option<NamedTempFile>> = filter_paths
        .iter()
        .map(|_| NamedTempFile::new().ok())
        .collect();

    let mut all_outputs: Vec<InstallStrategyOutput> = vec![];

    for (index, file) in filter_paths.iter().enumerate() {
        //  need a temp directory where to run the script
        let cwd = TempDir::new();
        let cwd_path = match cwd {
            Ok(ref c) => c.path().to_str().unwrap_or_default().to_string(),
            Err(_) => "".to_string(),
        };

        let mut permissions = file.as_file().metadata()?.permissions();
        permissions.set_mode(0o755);

        std::fs::set_permissions(file.path(), permissions).ok();

        let mut command = Command::new(file.path());
        command.current_dir(&cwd_path).stdout(Stdio::inherit());

        let output_path = match output_params_files[index] {
            Some(ref tmp) => tmp.path().to_str().unwrap_or_default().to_string(),
            None => "".to_string(),
        };

        command.env(defaults::SPM_OUTPUT_ENV_VAR_NAME, output_path);
        let mut child = command.spawn()?;

        child.wait().ok();

        //  read the output file if present
        let output_data = match output_params_files[index] {
            Some(ref tmp) => serde_json::from_reader(tmp).unwrap_or_default(),
            None => InstallStrategyOutput::default(),
        };

        consolidate_script_output(output_data, &mut all_outputs, &cwd_path);
    }

    match serde_json::to_string(&all_outputs) {
        Ok(json) => println!("All output: {}", json),
        Err(e) => eprintln!("All output error: {}", e),
    }

    Ok(())
}
