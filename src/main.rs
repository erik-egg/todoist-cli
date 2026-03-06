mod sync;

use clap::{Parser, Subcommand};
use reqwest::{Method, Url, blocking::Client, header::HeaderMap};
use serde_json::Value;

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

fn auth_headers() -> Result<HeaderMap, String> {
    let token = sync::get_token()
        .map_err(|e| format!("No Auth Token set: {e}\nPlease set it using `todo auth <token>`"))?;

    let mut headers = HeaderMap::new();
    headers.insert("Authorization", format!("Bearer {token}").parse().unwrap());

    Ok(headers)
}

fn resolve_task_id(id: usize) -> Result<String, String> {
    let list = sync::get_list("task_ids.txt").map_err(|e| {
        format!("No task list found: {e}\nPlease run `todo list` first to generate the task list.")
    })?;

    list.get(id).cloned().ok_or_else(|| {
        format!(
            "Task ID not found in list: {id}\nPlease ensure you are using a valid task ID from the most recent `todo list` output."
        )
    })
}

fn update_task(id: usize, method: Method, action_path: &str, success_message: &str) {
    let headers = match auth_headers() {
        Ok(headers) => headers,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let task_id = match resolve_task_id(id) {
        Ok(task_id) => task_id,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let api_link = format!("https://api.todoist.com/api/v1/tasks/{task_id}{action_path}");

    let _body = Client::new()
        .request(method, api_link)
        .headers(headers)
        .send()
        .unwrap();

    println!("{success_message}");
}

#[allow(clippy::too_many_lines)]
fn main() {
    let args = Args::parse();
    match args.command {
        Commands::Auth { token } => {
            if let Err(e) = sync::save_token(&token) {
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
            let headers = match auth_headers() {
                Ok(headers) => headers,
                Err(error) => {
                    eprintln!("{error}");
                    return;
                }
            };

            // parse inputs
            let limit = limit.unwrap_or(50);
            let filter = filter.or_else(|| {
                let mut filters = Vec::new();
                if let Some(search) = search {
                    filters.push(format!("search: {search}"));
                }
                if let Some(project) = project {
                    filters.push(format!("##{project}"));
                }
                if let Some(due) = due {
                    filters.push(format!("due: {due}"));
                }
                if let Some(before) = before {
                    filters.push(format!("due before: {before}"));
                }
                if let Some(after) = after {
                    filters.push(format!("due after: {after}"));
                }
                if let Some(priority) = priority {
                    filters.push(format!("p{priority}"));
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
            });

            // dbg!(&filter);

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

            if let Some(error) = body["error"].as_str() {
                eprintln!("API Error: {error}");
                return;
            }

            let today = chrono::Local::now().date_naive();
            let fallback_due = chrono::NaiveDate::from_ymd_opt(2222, 1, 1).unwrap();

            let mut ordered = Vec::new();
            let empty_tasks: &[Value] = &[];
            let tasks = body["results"]
                .as_array()
                .map_or(empty_tasks, Vec::as_slice);

            println!("{:<4}| {:<50} | Due Date", "ID", "Content - Description");
            println!("{}", "-".repeat(73));

            for task in tasks {
                // output line construction

                let content = task["content"].as_str().unwrap_or_default();
                let description = task["description"].as_str().unwrap_or_default();
                let task_due = task["due"]["date"].as_str().unwrap_or("none");
                let is_recurring = task["due"]["is_recurring"].as_bool().unwrap_or(false);

                let mut content_description = content.to_owned();

                if !description.is_empty() {
                    if !content_description.is_empty() {
                        content_description.push_str(" - ");
                    }
                    content_description.push_str(description);
                }

                let mut due_line = task_due.to_owned();

                let due =
                    chrono::NaiveDate::parse_from_str(task_due, "%Y-%m-%d").unwrap_or(fallback_due);

                if due < today {
                    due_line.push('⏰');
                }

                if is_recurring {
                    due_line.push('🔁');
                }

                let line = format!("{content_description:<50} | {due_line:<12}");

                ordered.push((
                    due,
                    line,
                    task["id"].as_str().unwrap_or_default().to_owned(),
                ));
            }

            ordered.sort_by_key(|k| k.0);
            for (idx, (_, line, _)) in ordered.iter().enumerate() {
                println!("{idx:<4}| {line}");
            }

            let list = ordered
                .into_iter()
                .map(|(_, _, id)| id)
                .collect::<Vec<String>>();

            sync::save_list(&list, "task_ids.txt").unwrap();
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
            let headers = match auth_headers() {
                Ok(headers) => headers,
                Err(error) => {
                    eprintln!("{error}");
                    return;
                }
            };

            // parse inputs
            if project.is_some() {
                todo!("project support not implemented yet");
            }

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

            let content = body["content"].as_str().unwrap_or_default();
            let description = body["description"].as_str().unwrap_or_default();
            let task_due = body["due"]["date"].as_str().unwrap_or("none");
            let is_recurring = body["due"]["is_recurring"].as_bool().unwrap_or(false);

            let mut content_description = content.to_owned();
            if !description.is_empty() {
                if !content_description.is_empty() {
                    content_description.push_str(" - ");
                }
                content_description.push_str(description);
            }

            let recurring_marker = if is_recurring { " 🔁" } else { "" };
            println!("0  | {content_description} (due: {task_due}{recurring_marker})");

            let list = vec![body["id"].as_str().unwrap_or_default().to_owned()];
            sync::save_list(&list, "task_ids.txt").unwrap();
        }

        Commands::Check { id } => {
            update_task(id, Method::POST, "/close", "Task closed successfully.");
        }

        Commands::Uncheck { id } => {
            update_task(id, Method::POST, "/reopen", "Task reopened successfully.");
        }

        Commands::Delete { id } => {
            update_task(id, Method::DELETE, "", "Task deleted successfully.");
        } // Commands::Projects {} => {
          //     todo!("Project listing not implemented yet.");
          // }
    }
}
