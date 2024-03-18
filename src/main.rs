mod cli;

fn main() {
    let command = cli::get_args();

    match command {
        cli::Commands::Install { application, infra } => {
            println!("{}", application);
            println!("{}", infra);
        }
        cli::Commands::Search { name, category } => {
            println!("{}", name);
            println!("{:?}", category);
        }
    }
}
