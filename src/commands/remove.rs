use std::{
    collections::HashMap, io::Write, os::unix::fs::PermissionsExt, path::Path, process::{Command, Stdio}
};

use tempfile::{NamedTempFile, TempDir};

use crate::{
    config::RegistryConfig,
    registry_api::{self, search::InstallStrategyResponse},
};

use super::file::{download_files, download_scripts};

pub struct Remove<'a, W: Write> {
    dry: bool,
    registry_config: &'a RegistryConfig,
    project: &'a String,
    env: &'a String,
    writer: &'a mut W,
}

impl<'a, W: Write> Remove<'a, W> {
    pub fn new(
        dry: bool,
        registry_config: &'a RegistryConfig,
        project: &'a String,
        env: &'a String,
        writer: &'a mut W,
    ) -> Self {
        Remove {
            dry,
            registry_config,
            project,
            env,
            writer,
        }
    }

    pub async fn run(self, strategy_name: String) -> anyhow::Result<()> {
        let result = registry_api::search::search_install_strategy(
            strategy_name,
            self.registry_config,
            self.writer,
        )
        .await;
        let strategy: InstallStrategyResponse;

        match result {
            Ok(str) => strategy = str,
            Err(err) => {
                eprintln!("{:?}", err);
                return Ok(());
            }
        };

        if self.dry {
            let _ = writeln!(
                self.writer,
                "Found install strategy: {}",
                serde_json::to_string_pretty(&strategy).unwrap_or_default()
            );

            return Ok(());
        }

        let _ = writeln!(self.writer, "Using strategy '{}'", strategy.title());

        //  need a temp directory where to run the script
        let cwd = TempDir::new();
        let cwd_path = match cwd {
            Ok(ref c) => c.path().to_str().unwrap_or_default().to_string(),
            Err(_) => "".to_string(),
        };

        let paths = download_scripts(&strategy.uninstall_scripts(), &cwd_path)
            .await
            .unwrap_or_default();

        let _ = download_files(&strategy.support_files(), &Path::new(&cwd_path)).await;

        let filter_paths: Vec<NamedTempFile> = paths.into_iter().filter_map(|p| p.ok()).collect();

        let mut env_vars: HashMap<String, String> = HashMap::new();

        //  add default env vars
        env_vars.insert(
            "SPM_PROJECT".to_string(),
            self.project.to_string(),
        );
        env_vars.insert("SPM_ENV".to_string(), self.env.to_string());

        for (_, file) in filter_paths.iter().enumerate() {
            let mut permissions = file.as_file().metadata()?.permissions();
            permissions.set_mode(0o755);

            std::fs::set_permissions(file.path(), permissions).ok();

            let mut command = Command::new(file.path());
            command.current_dir(&cwd_path).stdout(Stdio::inherit());

            //  add env vars to the scripts
            env_vars.clone().into_iter().for_each(|(k, v)| {
                command.env(k, v);
            });

            if self.dry == false {
                match command.spawn() {
                    Ok(mut child) => match child.wait() {
                        Ok(_) => println!("InstallStrategy remove successfully."),
                        Err(err) => println!("InstallStrategy remove failed with error {}", err),
                    },
                    Err(err) => {
                        eprintln!("Child process failed to start with error {}", err);
                    }
                };
            }
        }

        Ok(())
    }
}
