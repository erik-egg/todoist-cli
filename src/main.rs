mod utils;

use clap::{Parser, Subcommand};
use reqwest::header::HeaderMap;

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

    #[command(name = "list", alias = "l", alias = "get", alias = "g")]
    List {
        #[arg(short = 'p', long = "project")]
        project: Option<String>,

        #[arg(short = 'f', long = "filter")]
        filter: Option<String>,

        #[arg(short = 'l', long = "limit")]
        limit: Option<i32>,
    },

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

        Commands::List {
            project,
            filter,
            limit,
        } => {
            let token = match utils::auth::get_token() {
                Ok(token) => token,
                Err(e) => {
                    eprintln!("No Auth Token set: {e}\nPlease set it using `todo auth <token>`");
                    return;
                }
            };

            let api_link = "https://api.todoist.com/api/v1/tasks";
            let mut headers = HeaderMap::new();

            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );

            if let Some(project) = project {
                headers.insert("project_id", project.parse().unwrap());
            }
            if let Some(limit) = limit {
                headers.insert("limit", limit.to_string().parse().unwrap());
            }

            let body = reqwest::blocking::Client::new()
                .get(api_link)
                .headers(headers)
                .send()
                .unwrap()
                .json::<serde_json::Value>()
                .unwrap();

            for task in body["results"].as_array().unwrap() {
                println!(
                    "{}: {} - {} ",
                    task["id"], task["content"], task["description"]
                );
            }
        }

        Commands::Add {} => {
            todo!()
        }

        Commands::Check {} => {
            todo!()
        }

        Commands::Uncheck {} => {
            todo!()
        }

        Commands::Delete {} => {
            todo!()
        }
    }
}
