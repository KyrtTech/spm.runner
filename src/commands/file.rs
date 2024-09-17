use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use tempfile::NamedTempFile;

use crate::registry_api::install::InstallStrategySupportFile;

pub async fn download_scripts(
    scripts: &Vec<String>,
    parent: &String,
) -> anyhow::Result<Vec<Result<NamedTempFile, String>>> {
    let scripts_futures = scripts.iter().map(|url| async move {
        let response = reqwest::get(url).await.unwrap();

        if response.status().is_success() {
            match tempfile::Builder::new().tempfile_in(parent) {
                Ok(mut file) => {
                    let bytes = response.bytes().await.unwrap_or_default();

                    file.write_all(&bytes).ok();

                    return Ok(file);
                }
                Err(err) => Err(format!("{}", err)),
            }
        } else {
            eprintln!("Error downloading install script {}", url);
            Err(format!("Download install file failed {}", url))
        }
    });

    let script_paths = futures::future::join_all(scripts_futures).await;

    Ok(script_paths)
}

pub async fn download_files(
    files: &Vec<InstallStrategySupportFile>,
    parent: &Path,
) -> Vec<Result<(), String>> {
    let files_futures: Vec<_> = files
        .iter()
        .map(|remote_file| async {
            let response = reqwest::get(remote_file.access_url()).await.unwrap();

            if response.status().is_success() {
                let bytes = response.bytes().await.unwrap_or_default();

                let path = match std::path::absolute(parent.join(remote_file.local_path())) {
                    Ok(p) => p,
                    Err(err) => return Err(format!("Failed to compute absolute path {}", err)),
                };

                //  you are constrained by the power of the file system
                //  check if the path is outside of parent
                if !path.starts_with(parent) {
                    return Err(format!(
                        "Trying to break the file system containment {}",
                        path.display()
                    ));
                }

                if let Some(path) = path.parent() {
                    let _ = fs::create_dir_all(path);
                }

                let file = File::create(path);

                if let Ok(mut file) = file {
                    let _ = file.write(&bytes);
                }

                Ok(())
            } else {
                eprintln!("Could not download file {}", remote_file.access_url());

                Err(format!(
                    "Failed to download file {}.",
                    remote_file.access_url()
                ))
            }
        })
        .collect();

    let results: Vec<Result<(), String>> = futures::future::join_all(files_futures).await;

    return results;
}
