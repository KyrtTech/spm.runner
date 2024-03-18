use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Searches for a software application
    Search {
        /// Name of the software to search for
        #[arg(short, long)]
        name: String,

        /// Type of platform
        #[arg(short, long)]
        category: SearchCategory,
    },
    /// Installs a specified application on the desired infrastructure
    Install {
        /// Name of the application
        #[arg(short, long)]
        application: String,

        /// Name of the infra
        #[arg(short, long)]
        infra: String,
    },
}

/// Enum for the
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum SearchCategory {
    Platform,
    InstallStrategy,
    InfraProvider,
}

impl std::str::FromStr for SearchCategory {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "platform" => Ok(SearchCategory::Platform),
            "install-strategy" => Ok(SearchCategory::InstallStrategy),
            "infra-provider" => Ok(SearchCategory::InfraProvider),
            _ => Err("category not supported"),
        }
    }
}

pub fn get_args() -> Commands {
    let args = Args::parse();

    return args.command;
}
