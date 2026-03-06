mod utils;

use clap::{Parser, Subcommand};
use reqwest::{Url, blocking::Client, header::HeaderMap};
use serde_json::Value;

use crate::utils::sync;

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
        #[arg(short = 'f', long = "filter", conflicts_with_all = &["search", "project", "due", "before", "after", "overdue", "today", "tomorrow", "week", "recurring"])]
        filter: Option<String>,

        #[arg(short = 'l', long = "limit")]
        limit: Option<i32>,

        #[arg(short = 's', long = "search")]
        search: Option<String>,

        #[arg(short = 'p', long = "project")]
        project: Option<String>,

        #[arg(short = 'd', long = "due", conflicts_with_all = &["today", "tomorrow", "week", "month", "year"])]
        due: Option<String>,

        #[arg(short = 'b', long = "before")]
        before: Option<String>,

        #[arg(short = 'a', long = "after")]
        after: Option<String>,

        #[arg(short = 'P', long = "priority")]
        priority: Option<String>,

        #[arg(short = 'o', long = "overdue")]
        overdue: bool,

        #[arg(short = 't', long = "today", conflicts_with_all = &["due", "tomorrow", "week", "month", "year"])]
        today: bool,

        #[arg(short = 'T', long = "tomorrow", conflicts_with_all = &["due", "today", "week", "month", "year"])]
        tomorrow: bool,

        #[arg(short = 'w', long = "week", conflicts_with_all = &["due", "today", "tomorrow", "month", "year"])]
        week: bool,

        #[arg(short = 'm', long = "month", conflicts_with_all = &["due", "today", "tomorrow", "week", "year"])]
        month: bool,

        #[arg(short = 'y', long = "year", conflicts_with_all = &["due", "today", "tomorrow", "week", "month"])]
        year: bool,

        #[arg(short = 'r', long = "recurring")]
        recurring: bool,
    },

    #[command(name = "add", alias = "a")]
    Add {
        content: String,

        #[arg(short = 'p', long = "project")]
        project: Option<String>,

        #[arg(short = 'D', long = "description")]
        description: Option<String>,

        #[arg(short = 'd', long = "due", conflicts_with_all = &["today", "tomorrow", "week", "month", "year"])]
        due: Option<String>,

        #[arg(short = 'P', long = "priority")]
        priority: Option<String>,

        #[arg(short = 't', long = "today", conflicts_with_all = &["due", "tomorrow", "week", "month", "year"])]
        today: bool,

        #[arg(short = 'T', long = "tomorrow", conflicts_with_all = &["due", "today", "week", "month", "year"])]
        tomorrow: bool,

        #[arg(short = 'w', long = "week", conflicts_with_all = &["due", "today", "tomorrow", "month", "year"])]
        week: bool,

        #[arg(short = 'm', long = "month", conflicts_with_all = &["due", "today", "tomorrow", "week", "year"])]
        month: bool,

        #[arg(short = 'y', long = "year", conflicts_with_all = &["due", "today", "tomorrow", "week", "month"])]
        year: bool,
    },

    #[command(name = "check", alias = "c")]
    Check { id: usize },

    #[command(name = "uncheck", alias = "u")]
    Uncheck { id: usize },

    #[command(name = "delete", alias = "d")]
    Delete { id: usize },
    // #[command(name = "projects", alias = "p")]
    // Projects {},
}

fn main() {
    let args = Args::parse();
    match args.command {
        Commands::Auth { token } => {
            if let Err(e) = utils::sync::save_token(&token) {
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
            month,
            year,
            recurring,
        } => {
            let token = match utils::sync::get_token() {
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
                    filters.push(String::from("due before: next week"));
                }
                if month {
                    filters.push(String::from("due before: next month"));
                }
                if year {
                    filters.push(String::from("due before: next year"));
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

            let today = chrono::Local::now().date_naive();

            let mut ordered = Vec::new();
            let tasks = match body["results"].as_array() {
                Some(tasks) => tasks,
                None => &FALLBACK_VEC,
            };

            println!(
                "{:<4}| {:<50} | {}",
                "ID", "Content - Description", "Due Date"
            );
            println!("{}", "-".repeat(73));

            for task in tasks {
                // output line construction

                let content = task["content"].as_str().unwrap_or_default();
                let description = task["description"].as_str().unwrap_or_default();
                let task_due = task["due"]["date"].as_str().unwrap_or_else(|| &"none");
                let is_recurring = task["due"]["is_recurring"].as_bool().unwrap_or(false);

                let mut content_description = content.to_owned();

                if !description.is_empty() {
                    if !content_description.is_empty() {
                        content_description.push_str(" - ");
                    }
                    content_description.push_str(description);
                }

                let mut due_line = task_due.to_owned();

                let due = chrono::NaiveDate::parse_from_str(task_due, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(2222, 1, 1).unwrap());

                if due < today {
                    due_line.push_str("⏰");
                }

                if is_recurring {
                    due_line.push_str("🔁");
                }

                let line = format!("{:<50} | {:<12}", content_description, due_line);

                ordered.push((
                    due,
                    line,
                    task["id"].as_str().unwrap_or_default().to_owned(),
                ));
            }

            ordered.sort_by_key(|k| k.0);
            for (idx, (_, line, _)) in ordered.iter().enumerate() {
                println!("{:<4}| {}", idx, line);
            }

            let list = &ordered
                .into_iter()
                .map(|(_, _, id)| id)
                .collect::<Vec<String>>();

            sync::save_list(list, "task_ids.txt").unwrap();
        }

        Commands::Add {
            content,
            project,
            description,
            due,
            priority,
            today,
            tomorrow,
            week,
            month,
            year,
        } => {
            let token = match utils::sync::get_token() {
                Ok(token) => token,
                Err(e) => {
                    eprintln!("No Auth Token set: {e}\nPlease set it using `todo auth <token>`");
                    return;
                }
            };

            // parse inputs
            if project.is_some() {
                todo!("project support not implemented yet");
            }

            // call API
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );

            let api_link = "https://api.todoist.com/api/v1/tasks";

            let mut url = Url::parse(api_link).unwrap();
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("content", &content);
                // if let Some(project) = project {
                //     query.append_pair("project_id", &project);
                // }
                if let Some(description) = description {
                    query.append_pair("description", &description);
                }
                if let Some(due) = due {
                    query.append_pair("due_string", &due);
                }
                if let Some(priority) = priority {
                    query.append_pair("priority", &priority);
                }
                if today {
                    query.append_pair("due_string", "today");
                }
                if tomorrow {
                    query.append_pair("due_string", "tomorrow");
                }
                if week {
                    query.append_pair("due_string", "in 7 days");
                }
                if month {
                    query.append_pair("due_string", "in 1 month");
                }
                if year {
                    query.append_pair("due_string", "in 1 year");
                }
            }

            let body = Client::new()
                .post(url)
                .headers(headers)
                .send()
                .unwrap()
                .json::<Value>()
                .unwrap();

            // dbg!(&body);

            if let Some(error) = body["error"].as_str() {
                eprintln!("API Error: {error}");
                return;
            }

            let mut line = String::new();

            line.push_str("0  | ");

            let content = body["content"].as_str().unwrap_or_default();
            let description = body["description"].as_str().unwrap_or_default();
            let task_due = body["due"]["date"].as_str().unwrap_or_else(|| &"none");
            let is_recurring = body["due"]["is_recurring"].as_bool().unwrap_or(false);

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

            utils::sync::save_list(
                &vec![body["id"].as_str().unwrap_or_default().to_owned()],
                "task_ids.txt",
            )
            .unwrap();
        }

        Commands::Check { id } => {
            let token = match utils::sync::get_token() {
                Ok(token) => token,
                Err(e) => {
                    eprintln!("No Auth Token set: {e}\nPlease set it using `todo auth <token>`");
                    return;
                }
            };

            // call API
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );

            let list = match utils::sync::get_list("task_ids.txt") {
                Ok(list) => list,
                Err(e) => {
                    eprintln!(
                        "No task list found: {e}\nPlease run `todo list` first to generate the task list."
                    );
                    return;
                }
            };

            // dbg!(&list);

            let id = match list
                .iter()
                .enumerate()
                .find(|(idx, _)| *idx == id)
                .map(|(_, task_id)| task_id.to_owned())
            {
                Some(idx) => idx,
                None => {
                    eprintln!(
                        "Task ID not found in list: {}\nPlease ensure you are using a valid task ID from the most recent `todo list` output.",
                        id
                    );
                    return;
                }
            };

            let api_link = format!("https://api.todoist.com/api/v1/tasks/{}/close", id);
            // dbg!(&api_link);

            let _body = Client::new()
                .post(api_link)
                .headers(headers)
                .send()
                .unwrap();

            // dbg!(&_body);

            println!("Task closed successfully.");
        }

        Commands::Uncheck { id } => {
            let token = match utils::sync::get_token() {
                Ok(token) => token,
                Err(e) => {
                    eprintln!("No Auth Token set: {e}\nPlease set it using `todo auth <token>`");
                    return;
                }
            };

            // call API
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );

            let list = match utils::sync::get_list("task_ids.txt") {
                Ok(list) => list,
                Err(e) => {
                    eprintln!(
                        "No task list found: {e}\nPlease run `todo list` first to generate the task list."
                    );
                    return;
                }
            };

            let id = match list
                .iter()
                .enumerate()
                .find(|(idx, _)| *idx == id)
                .map(|(_, task_id)| task_id.to_owned())
            {
                Some(idx) => idx,
                None => {
                    eprintln!(
                        "Task ID not found in list: {}\nPlease ensure you are using a valid task ID from the most recent `todo list` output.",
                        id
                    );
                    return;
                }
            };

            let api_link = format!("https://api.todoist.com/api/v1/tasks/{}/reopen", id);

            let _body = Client::new()
                .post(api_link)
                .headers(headers)
                .send()
                .unwrap();

            // dbg!(&_body);

            println!("Task reopened successfully.");
        }

        Commands::Delete { id } => {
            let token = match utils::sync::get_token() {
                Ok(token) => token,
                Err(e) => {
                    eprintln!("No Auth Token set: {e}\nPlease set it using `todo auth <token>`");
                    return;
                }
            };

            // call API
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );

            let list = match utils::sync::get_list("task_ids.txt") {
                Ok(list) => list,
                Err(e) => {
                    eprintln!(
                        "No task list found: {e}\nPlease run `todo list` first to generate the task list."
                    );
                    return;
                }
            };

            let id = match list
                .iter()
                .enumerate()
                .find(|(idx, _)| *idx == id)
                .map(|(_, task_id)| task_id.to_owned())
            {
                Some(idx) => idx,
                None => {
                    eprintln!(
                        "Task ID not found in list: {}\nPlease ensure you are using a valid task ID from the most recent `todo list` output.",
                        id
                    );
                    return;
                }
            };

            let api_link = format!("https://api.todoist.com/api/v1/tasks/{}", id);

            let _body = Client::new()
                .delete(api_link)
                .headers(headers)
                .send()
                .unwrap();

            // dbg!(&_body);

            println!("Task deleted successfully.");
        } // Commands::Projects {} => {
          //     todo!("Project listing not implemented yet.");
          // }
    }
}
