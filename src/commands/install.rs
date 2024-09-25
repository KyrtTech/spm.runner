use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
};

use heck::ToShoutySnakeCase;

use tempfile::{NamedTempFile, TempDir};

use crate::{config::AppConfig, defaults};
use crate::{config::RegistryConfig, optional_params::UserOptionalParams};
use crate::{
    interactive_protocol,
    optional_params::UriEntry,
    registry_api::install::{find_install_strategy, PlanInstallStrategyResponse},
};

use super::file::{download_files, download_scripts};

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

fn add_env_user_params_for_strategy(
    strategy_params: &HashMap<String, UriEntry>,
    env: &mut HashMap<String, String>,
) {
    strategy_params.keys().into_iter().for_each(|key| {
        let param = match strategy_params.get(key) {
            Some(param) => param,
            None => {
                eprintln!("Could not get user param {}", key);
                return;
            }
        };

        if param.is_file() {
            return;
        }

        let val = param.payload();

        let fragments = key.split("/").collect::<Vec<&str>>();
        let name = fragments.last().unwrap_or(&"");

        match param.is_reference() {
            true => {
                let env_name = match val.split("/").collect::<Vec<&str>>().last() {
                    Some(referenced_env) => referenced_env.to_string().to_shouty_snake_case(),
                    None => return,
                };

                match env.get(&env_name) {
                    Some(env_var) => {
                        env.insert(name.to_string().to_shouty_snake_case(), env_var.to_string());
                        ()
                    }
                    None => {
                        eprintln!("Could not get env var {}", env_name);
                        ()
                    }
                }
            }
            false => {
                env.insert(name.to_string().to_shouty_snake_case(), val);
                ()
            }
        }
    })
}

fn move_file_user_params_for_strategy(
    strategy_params: &HashMap<String, UriEntry>,
) -> HashMap<String, NamedTempFile> {
    let mut ret: HashMap<String, NamedTempFile> = HashMap::new();

    strategy_params.keys().into_iter().for_each(|key| {
        let param = match strategy_params.get(key) {
            Some(param) => param,
            None => {
                eprintln!("Could not get user param {}", key);
                return;
            }
        };

        if param.is_file() == false {
            return;
        }

        let fragments = key.split("/").collect::<Vec<&str>>();
        let name = fragments.last().unwrap_or(&"");
        let some_file = NamedTempFile::new();

        if let Ok(file) = some_file {
            param.write_to_file(file.path().to_str().unwrap_or_default());

            ret.insert(name.to_string().to_shouty_snake_case(), file);
        }
    });

    return ret;
}

pub struct RunArguments<'a> {
    pub application: &'a str,
    pub infra: &'a str,
    pub user_params: &'a UserOptionalParams,
}

pub struct Install<'a, W: Write> {
    app_config: AppConfig<'a, 'a>,
    registry_config: &'a RegistryConfig,
    writer: &'a mut W,
}

impl<'a, W: Write> Install<'a, W> {
    pub fn new(
        app_config: AppConfig<'a, 'a>,
        registry: &'a RegistryConfig,
        writer: &'a mut W,
    ) -> Self {
        Install {
            app_config,
            registry_config: registry,
            writer,
        }
    }

    async fn run_strategy(
        &mut self,
        strategy: &PlanInstallStrategyResponse,
        env_vars: &HashMap<String, String>,
    ) -> anyhow::Result<Vec<InstallStrategyOutput>> {
        let _ = writeln!(self.writer, "Using strategy '{}'", strategy.title());

        //  need a temp directory where to run the script
        let cwd = TempDir::new();
        let cwd_path = match cwd {
            Ok(ref c) => c.path().to_str().unwrap_or_default().to_string(),
            Err(_) => "".to_string(),
        };

        let paths = download_scripts(&strategy.scripts(), &cwd_path)
            .await
            .unwrap_or_default();

        //  download the support files
        let _ = download_files(&strategy.support_files(), &Path::new(&cwd_path)).await;

        let filter_paths: Vec<NamedTempFile> = paths.into_iter().filter_map(|p| p.ok()).collect();

        let output_params_files: Vec<Option<NamedTempFile>> = filter_paths
            .iter()
            .map(|_| NamedTempFile::new().ok())
            .collect();

        let mut all_outputs: Vec<InstallStrategyOutput> = vec![];

        for (index, file) in filter_paths.iter().enumerate() {
            let mut permissions = file.as_file().metadata()?.permissions();
            permissions.set_mode(0o755);

            std::fs::set_permissions(file.path(), permissions).ok();

            let mut command = Command::new(file.path());
            command.current_dir(&cwd_path).stdout(Stdio::inherit());

            let output_path = match output_params_files[index] {
                Some(ref tmp) => tmp.path().to_str().unwrap_or_default().to_string(),
                None => "".to_string(),
            };

            command.env(defaults::SPM_OUTPUT_ENV_VAR_NAME, &output_path);

            if self.app_config.verbose() {
                println!("Writing to file {}", output_path);
                println!("{:?}", env_vars)
            }

            //  add env vars to the scripts
            env_vars.into_iter().for_each(|(k, v)| {
                command.env(k, v);
            });

            if self.app_config.dry_run() == false {
                match command.spawn() {
                    Ok(mut child) => match child.wait() {
                        Ok(_) => println!("InstallStrategy executed successfully."),
                        Err(err) => println!("InstallStrategy failed with error {}", err),
                    },
                    Err(err) => {
                        eprintln!("Child process failed to start with error {}", err);
                    }
                };
            }

            //  read the output file if present
            let output_data = match output_params_files[index] {
                Some(ref tmp) => serde_json::from_reader(tmp).unwrap_or_default(),
                None => InstallStrategyOutput::default(),
            };

            let _ = consolidate_script_output(output_data, &mut all_outputs, &cwd_path);
        }

        return Ok(all_outputs);
    }

    async fn communicate_plan(&mut self, strategies: &Vec<PlanInstallStrategyResponse>) {
        if self.app_config.interactive() {
            if let Some(comm) = self.app_config.comm() {
                let _ = comm
                    .sender()
                    .send(interactive_protocol::Message::PlanDescription(
                        serde_json::to_string(&strategies).unwrap(),
                    ))
                    .await;
                loop {
                    let message = comm.receiver().recv().await;

                    if let Some(message) = message {
                        match message {
                            interactive_protocol::Message::StartJob => return,
                            _ => continue,
                        }
                    } else {
                        std::process::exit(2);
                    }
                }
            }
        }
    }

    async fn communicate_start_env(
        &mut self,
        envs: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        if !self.app_config.interactive() {
            return HashMap::new();
        }

        let data = Arc::new(envs.clone());

        if let Some(comm) = self.app_config.comm() {
            let _ = comm
                .sender()
                .send(interactive_protocol::Message::PlanEnvs(data))
                .await;

            let maybe_message = comm.receiver().recv().await;

            if let Some(message) = maybe_message {
                match message {
                    interactive_protocol::Message::PlanEnvsResponse(arc) => return (*arc).clone(),
                    _ => (),
                }
            }
        }

        HashMap::new()
    }

    async fn communicate_strategy_env(&mut self, index: u16, envs: &HashMap<String, String>) {
        if !self.app_config.interactive() {
            return;
        }

        let data = Arc::new(envs.clone());

        if let Some(comm) = self.app_config.comm() {
            let _ = comm
                .sender()
                .send(interactive_protocol::Message::StrategyEnv(index, data))
                .await;
        }
    }

    pub async fn run(mut self, args: RunArguments<'_>) -> anyhow::Result<HashMap<String, String>> {
        let mut strategies = vec![];
        let mut env_vars = HashMap::<String, String>::new();

        //  add default env vars
        env_vars.insert(
            "SPM_PROJECT".to_string(),
            self.app_config.project().to_string(),
        );
        env_vars.insert("SPM_ENV".to_string(), self.app_config.env().to_string());

        match find_install_strategy(
            args.application,
            args.infra,
            self.registry_config,
            self.writer,
        )
        .await
        {
            Ok(str) => strategies = str,
            Err(err) => eprintln!("{}", err),
        }

        //  want to pass this to the web server
        //  and wait for a response from the user
        self.communicate_plan(&strategies).await;

        //  send default envs
        self.communicate_start_env(&env_vars).await;

        if self.app_config.dry_run() {
            let _ = writeln!(
                self.writer,
                "Using env vars: {}",
                serde_json::to_string_pretty(&env_vars).unwrap_or_default()
            );
            let _ = writeln!(
                self.writer,
                "{}",
                serde_json::to_string_pretty(&strategies).unwrap_or_default()
            );
        }

        if strategies.len() == 0 {
            let _ = writeln!(self.writer, "No valid install strategies found");
            return Ok(env_vars);
        }

        //  clean up the output folder first
        let _ = fs::remove_dir_all(defaults::OUTPUT_FOLDER);

        //  init persistent output directory
        fs::create_dir_all(defaults::OUTPUT_FOLDER)?;

        let mut index = 0;

        for strategy in strategies {
            if let Some(download_url) = &strategy.download_url() {
                env_vars.insert(String::from("DOWNLOAD_URL"), download_url.clone());
            }

            save_env_vars(&env_vars, index);

            self.communicate_strategy_env(index, &env_vars).await;

            let mut env_vars_with_user = env_vars.clone();

            //  parse env user params
            if let Some(strategy_params) = &args.user_params.get(strategy.uri()) {
                add_env_user_params_for_strategy(&strategy_params, &mut env_vars_with_user);

                let user_params = move_file_user_params_for_strategy(&strategy_params);

                //  will move temp files in this vec
                //  we only send the path, so the file
                //  would go out of scope and be deleted
                let mut temp_files: Vec<NamedTempFile> = vec![];

                //  extend env_vars with the user provided
                env_vars_with_user.extend(user_params.into_iter().map(|(key, file)| {
                    let path = file.path().to_str().unwrap_or_default().to_string();
                    temp_files.push(file);
                    return (key, path);
                }));
            }

            for param in strategy.dependency_params() {
                let short_name = match param.spm_param().split("/").last() {
                    Some(name) => name,
                    None => "empty",
                };

                let shouty_name = short_name.to_shouty_snake_case();

                if self.app_config.use_min_specs() {
                    env_vars_with_user
                        .insert(format!("SPM_{shouty_name}"), param.min().to_string());
                } else {
                    env_vars_with_user
                        .insert(format!("SPM_{shouty_name}"), param.rec().to_string());
                }
            }

            let all_outputs = self
                .run_strategy(&strategy, &env_vars_with_user)
                .await
                .unwrap_or_default();

            if all_outputs.len() != 0 {
                env_vars.extend(
                    (&all_outputs[0].output_params)
                        .into_iter()
                        .map(|(k, v)| (k.to_shouty_snake_case(), v.clone())),
                );

                match serde_json::to_string(&all_outputs) {
                    Ok(json) => writeln!(self.writer, "All output: {}", json).unwrap_or(()),
                    Err(e) => writeln!(self.writer, "All output error: {}", e).unwrap_or(()),
                }
            }

            index += 1;
        }

        let _ = writeln!(self.writer, "Final env vars: ");
        let _ = writeln!(
            self.writer,
            "{}",
            serde_json::to_string_pretty(&env_vars).unwrap_or_default()
        );

        save_env_vars(&env_vars, index);

        Ok(env_vars)
    }
}
