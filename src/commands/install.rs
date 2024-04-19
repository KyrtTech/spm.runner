use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

use heck::ToShoutySnakeCase;

use tempfile::{NamedTempFile, TempDir};

use crate::defaults;
use crate::{config::RegistryConfig, optional_params::UserOptionalParams};

#[derive(serde::Serialize, serde::Deserialize)]
struct InstallStrategyArgument {
    name: String,
    param: String,
    description: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct InstallStrategyResponse {
    title: String,
    description: String,
    uri: String,
    scripts: Vec<String>,
    #[serde(rename = "downloadURL")]
    download_url: Option<String>,
    runner: Option<String>,
    #[serde(rename = "optionalParams")]
    optional_params: Vec<InstallStrategyArgument>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
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

async fn do_install<W: Write>(
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

fn consolidate_script_output(
    data: InstallStrategyOutput,
    consolidate: &mut Vec<InstallStrategyOutput>,
    script_folder: &str,
) -> anyhow::Result<()> {
    let mut new_data = data.clone();

    //  we need to save files to persistent locations
    if let Some(ssh_key_file) = data.output_params.get("sshKey") {
        let destination = format!("{}/{}.pem", defaults::OUTPUT_FOLDER, consolidate.len());

        let _ = std::fs::copy(format!("{}/{}", script_folder, ssh_key_file), &destination);

        new_data.output_params.insert(
            "sshKey".to_string(),
            fs::canonicalize(destination)?
                .to_str()
                .unwrap_or_default()
                .to_string(),
        );
    }

    consolidate.push(new_data);

    Ok(())
}

async fn run_strategy<W: Write>(
    strategy: &InstallStrategyResponse,
    env_vars: &HashMap<String, String>,
    writer: &mut W,
) -> anyhow::Result<Vec<InstallStrategyOutput>> {
    let _ = writeln!(writer, "Using strategy '{}'", strategy.title);

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
        //  add env vars to the scripts
        env_vars.into_iter().for_each(|(k, v)| {
            command.env(k, v);
        });

        let mut child = command.spawn()?;

        child.wait().ok();

        //  read the output file if present
        let output_data = match output_params_files[index] {
            Some(ref tmp) => serde_json::from_reader(tmp).unwrap_or_default(),
            None => InstallStrategyOutput::default(),
        };

        let _ = consolidate_script_output(output_data, &mut all_outputs, &cwd_path);
    }

    return Ok(all_outputs);
}

fn save_env_vars(env: &HashMap<String, String>, index: u16) {
    if let Ok(serialized) = serde_json::to_string(env) {
        if let Ok(mut file) = File::create(format!(
            "{}/{}.env.json",
            defaults::OUTPUT_FOLDER,
            index.to_string()
        )) {
            let _ = file.write_all(serialized.as_bytes());
        }
    }
}

fn move_user_params_for_strategy(
    params: &UserOptionalParams,
    name: &str,
) -> HashMap<String, NamedTempFile> {
    let default = HashMap::new();
    let strategy_params = params.get(name).unwrap_or(&default);

    let mut ret: HashMap<String, NamedTempFile> = HashMap::new();

    strategy_params.keys().into_iter().for_each(|key| {
        let fragments = key.split("/").collect::<Vec<&str>>();
        let name = fragments.last().unwrap_or(&"");
        let some_file = NamedTempFile::new();

        if let Ok(file) = some_file {
            let param_maybe = strategy_params.get(key);

            if let Some(param) = param_maybe {
                param.write_to_file(file.path().to_str().unwrap_or_default());
            }

            ret.insert(name.to_string().to_shouty_snake_case(), file);
        }
    });

    return ret;
}

pub struct RunArguments<'a> {
    pub application: &'a str,
    pub infra: &'a str,
    pub dry: bool,
    pub user_params: &'a UserOptionalParams,
    pub registry_config: &'a RegistryConfig,
}

pub async fn run<W: Write>(
    args: RunArguments<'_>,
    writer: &mut W,
) -> anyhow::Result<HashMap<String, String>> {
    let mut strategies = vec![];
    let mut env_vars = HashMap::<String, String>::new();

    match do_install(args.application, args.infra, args.registry_config, writer).await {
        Ok(str) => strategies = str,
        Err(err) => eprintln!("{}", err),
    }

    if args.dry {
        let _ = writeln!(
            writer,
            "{}",
            serde_json::to_string_pretty(&strategies).unwrap_or_default()
        );
        return Ok(env_vars);
    }

    if strategies.len() == 0 {
        let _ = writeln!(writer, "No valid install strategies found");
        return Ok(env_vars);
    }

    //  init persistent output directory
    fs::create_dir_all(defaults::OUTPUT_FOLDER)?;

    let mut index = 0;

    for strategy in strategies {
        if let Some(download_url) = &strategy.download_url {
            env_vars.insert(String::from("DOWNLOAD_URL"), download_url.clone());
        }

        save_env_vars(&env_vars, index);

        let user_params = move_user_params_for_strategy(&args.user_params, &strategy.uri);

        //  will move temp files in this vec
        //  we only send the path, so the file
        //  would go out of scope and be deleted
        let mut temp_files: Vec<NamedTempFile> = vec![];

        //  extend env_vars with the user provided
        let mut env_vars_with_user = env_vars.clone();
        env_vars_with_user.extend(user_params.into_iter().map(|(key, file)| {
            let path = file.path().to_str().unwrap_or_default().to_string();
            temp_files.push(file);
            return (key, path);
        }));

        let all_outputs = run_strategy(&strategy, &env_vars_with_user, writer)
            .await
            .unwrap_or_default();

        if all_outputs.len() != 0 {
            env_vars.extend(
                (&all_outputs[0].output_params)
                    .into_iter()
                    .map(|(k, v)| (k.to_shouty_snake_case(), v.clone())),
            );

            match serde_json::to_string(&all_outputs) {
                Ok(json) => writeln!(writer, "All output: {}", json).unwrap_or(()),
                Err(e) => writeln!(writer, "All output error: {}", e).unwrap_or(()),
            }
        }

        index += 1;
    }

    let _ = writeln!(writer, "Final env vars: ");
    let _ = writeln!(
        writer,
        "{}",
        serde_json::to_string_pretty(&env_vars).unwrap_or_default()
    );

    save_env_vars(&env_vars, index);

    Ok(env_vars)
}
