mod cli;
mod config;
mod defaults;

fn do_search(name: &str, category: cli::SearchCategory) {
    println!("Doing search for: {}, {:?}", name, category);
}

fn do_install(app_name: &str, infra: &str) {
    println!("Doing install for: {}, {}", app_name, infra);
}

fn main() {
    let registry_config = config::RegistryConfig::default().update_from_env();

    println!("Using registry config {:?}", registry_config);

    let command = cli::get_args();

    match command {
        cli::Commands::Install { application, infra } => {
            do_install(&application, &infra);
        }
        cli::Commands::Search { name, category } => {
            do_search(&name, category);
        }
    }
}
