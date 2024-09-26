use std::{collections::HashMap, fs::File, io::Write};

use heck::ToShoutySnakeCase;
use tempfile::NamedTempFile;

use crate::{defaults, optional_params::UriEntry};

pub fn save_env_vars(env: &HashMap<String, String>, index: u16) {
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

pub fn add_env_user_params_for_strategy(
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

pub fn move_file_user_params_for_strategy(
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
