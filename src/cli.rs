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

        /// Optional params file
        #[arg(short, long, default_value = "")]
        user_params: String,

        /// Dry run, returns the plan, but doesn't run it
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        dry: bool,

        /// Project name
        #[arg(short, long, default_value = "SMP")]
        project: String,

        /// Environment
        #[arg(short, long, default_value = "dev")]
        env: String,
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

impl std::fmt::Display for SearchCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let var_str = match self {
            Self::InfraProvider => "infra-provider",
            Self::Platform => "platform",
            Self::InstallStrategy => "install-strategy",
        };

        write!(f, "{}", var_str)
    }
}

pub fn get_args() -> Commands {
    let args = Args::parse();

    return args.command;
}
