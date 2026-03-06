mod utils;

use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use reqwest::{Url, blocking::Client, header::HeaderMap};
use serde_json::Value;

const FALLBACK_NAIVE_DATE: Option<NaiveDate> = NaiveDate::from_ymd_opt(2222, 1, 1);
const FALLBACK_VEC: Vec<Value> = Vec::new();

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
        #[arg(short = 'f', long = "filter", conflicts_with_all = &["search", "project", "due", "before", "after", "overdue", "today", "tomorrow"])]
        filter: Option<String>,

        #[arg(short = 'l', long = "limit")]
        limit: Option<i32>,

        #[arg(short = 's', long = "search")]
        search: Option<String>,

        #[arg(short = 'p', long = "project")]
        project: Option<String>,

        #[arg(short = 'd', long = "due")]
        due: Option<String>,

        #[arg(short = 'b', long = "before")]
        before: Option<String>,

        #[arg(short = 'a', long = "after")]
        after: Option<String>,

        #[arg(short = 'P', long = "priority")]
        priority: Option<String>,

        #[arg(short = 'o', long = "overdue")]
        overdue: bool,

        #[arg(short = 't', long = "today")]
        today: bool,

        #[arg(short = '1', long = "tomorrow")]
        tomorrow: bool,

        #[arg(short = 'w', long = "week")]
        week: bool,

        #[arg(short = 'r', long = "recurring")]
        recurring: bool,
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
            filter,
            limit,
            search,
            project,
            due,
            before,
            after,
            priority,
            overdue,
            today,
            tomorrow,
            week,
            recurring,
        } => {
            let token = match utils::auth::get_token() {
                Ok(token) => token,
                Err(e) => {
                    eprintln!("No Auth Token set: {e}\nPlease set it using `todo auth <token>`");
                    return;
                }
            };

            // parse inputs
            let limit = limit.unwrap_or(50);
            let filter = if filter.is_none() {
                let mut filters = Vec::new();
                if let Some(search) = search {
                    filters.push(format!("search: {}", search));
                }
                if let Some(project) = project {
                    filters.push(format!("##{}", project));
                }
                if let Some(due) = due {
                    filters.push(format!("due: {}", due));
                }
                if let Some(before) = before {
                    filters.push(format!("due before: {}", before));
                }
                if let Some(after) = after {
                    filters.push(format!("due after: {}", after));
                }
                if let Some(priority) = priority {
                    filters.push(format!("p{}", priority));
                }
                if overdue {
                    filters.push(String::from("overdue"));
                }
                if today {
                    filters.push(String::from("due: today"));
                }
                if tomorrow {
                    filters.push(String::from("due: tomorrow"));
                }
                if week {
                    filters.push(String::from("due: this week"));
                }
                if recurring {
                    filters.push(String::from("recurring"));
                }
                if filters.is_empty() {
                    None
                } else {
                    Some(filters.join(" & "))
                }
            } else {
                filter
            };

            // dbg!(&filter);

            // call API
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );

            let api_link = if filter.is_none() {
                // dbg!("no filter, using tasks endpoint");
                "https://api.todoist.com/api/v1/tasks"
            } else {
                // dbg!("filter provided, using filter endpoint");
                "https://api.todoist.com/api/v1/tasks/filter"
            };

            let mut url = Url::parse(api_link).unwrap();
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("limit", &limit.to_string());
                if let Some(filter) = filter {
                    query.append_pair("query", &filter);
                }
            }

            let body = Client::new()
                .get(url)
                .headers(headers)
                .send()
                .unwrap()
                .json::<Value>()
                .unwrap();

            // dbg!(&body);
            println!("");

            if let Some(error) = body["error"].as_str() {
                eprintln!("API Error: {error}");
                return;
            }

            // parse and print results
            let mut count = 1;

            let tasks = match body["results"].as_array() {
                Some(tasks) => tasks,
                None => &FALLBACK_VEC,
            };
            for task in tasks {
                // limit filter
                count += 1;

                // output line construction
                let mut line = String::new();

                if task["checked"].as_bool().unwrap_or(false) {
                    line.push_str("[x] ");
                } else {
                    line.push_str("[ ] ");
                }

                let content = task["content"].as_str().unwrap_or_default();
                let description = task["description"].as_str().unwrap_or_default();
                let task_due = task["due"]["date"].as_str().unwrap();
                let is_recurring = task["due"]["is_recurring"].as_bool().unwrap_or(false);

                if !content.is_empty() {
                    line.push_str(content);
                }

                if !description.is_empty() {
                    line.push_str(" - ");
                    line.push_str(description);
                }

                line.push_str(&format!(" (due: {}", task_due));

                if is_recurring {
                    line.push_str(" 🔁");
                }

                line.push_str(")");

                println!("{}", line);

                if count > limit {
                    break;
                }
            }
        }

        Commands::Projects {} => {
            todo!()
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
