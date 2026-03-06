mod utils;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "todo", about = "A simple Todoist API application")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Auth {
        #[arg(required = true)]
        token: String,
    },

    #[command(name = "list", alias = "l")]
    List {},

    #[command(name = "add", alias = "a")]
    Add {},

    #[command(name = "check", alias = "c")]
    Check {},

    #[command(name = "uncheck", alias = "u")]
    Uncheck {},

    #[command(name = "delete", alias = "d")]
    Delete {},
}

fn main() {
    let args = Args::parse();
    match args.command {
        Commands::Auth { token } => {
            if let Err(e) = utils::auth::save_token(&token) {
                eprintln!("Error saving token: {e}");
            } else {
                println!("Successfully saved token.");
            }
        }
        Commands::List {} => todo!(),
        Commands::Add {} => todo!(),
        Commands::Check {} => todo!(),
        Commands::Uncheck {} => todo!(),
        Commands::Delete {} => todo!(),
    }
}
