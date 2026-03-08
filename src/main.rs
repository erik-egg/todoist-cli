mod sync;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use reqwest::{
    Method, Url,
    blocking::{Client, Response},
    header::HeaderMap,
};
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(
    name = "todo",
    version,
    about = "Manage Todoist tasks from the terminal",
    long_about = "A Todoist CLI for listing, creating, and updating tasks.\n\
IDs used by `check`, `uncheck`, and `delete` come from the most recent `todo list` output."
)]
struct Args {
    /// Subcommand to run.
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Save your Todoist API token locally.
    Auth {
        /// Todoist API token.
        ///
        /// Generate one from Todoist settings and pass it as plain text:
        /// `todo auth <token>`.
        #[arg(required = true)]
        token: String,
    },

    /// List tasks with optional filters.
    #[command(name = "list", alias = "l", alias = "get", alias = "g")]
    List {
        /// Raw Todoist query string.
        ///
        /// This is passed directly as `query` to the Todoist filter endpoint.
        /// Cannot be used with the structured filter flags below.
        #[arg(short = 'f', long = "filter", conflicts_with_all = &["search", "project", "due", "before", "after", "overdue", "today", "tomorrow", "week", "recurring"])]
        filter: Option<String>,

        /// Maximum number of tasks to fetch.
        ///
        /// Defaults to 25 when omitted.
        #[arg(short = 'l', long = "limit")]
        limit: Option<i32>,

        /// Full-text search term.
        #[arg(short = 's', long = "search")]
        search: Option<String>,

        /// Project name used in filter queries.
        ///
        /// The value is translated to `##<project>`.
        #[arg(short = 'p', long = "project")]
        project: Option<String>,

        /// Natural-language due date filter (for example: "today", "next Friday").
        #[arg(short = 'd', long = "due", conflicts_with_all = &["today", "tomorrow", "week", "month", "year"])]
        due: Option<String>,

        /// Show tasks due before this date expression.
        #[arg(short = 'b', long = "before")]
        before: Option<String>,

        /// Show tasks due after this date expression.
        #[arg(short = 'a', long = "after")]
        after: Option<String>,

        /// Filter by priority (1-4).
        #[arg(short = 'P', long = "priority")]
        priority: Option<String>,

        /// Include only overdue tasks.
        #[arg(short = 'o', long = "overdue")]
        overdue: bool,

        /// Include tasks due today.
        #[arg(short = 't', long = "today", conflicts_with_all = &["due", "tomorrow", "week", "month", "year"])]
        today: bool,

        /// Include tasks due tomorrow.
        #[arg(short = 'T', long = "tomorrow", conflicts_with_all = &["due", "today", "week", "month", "year"])]
        tomorrow: bool,

        /// Include tasks due before next week.
        #[arg(short = 'w', long = "week", conflicts_with_all = &["due", "today", "tomorrow", "month", "year"])]
        week: bool,

        /// Include tasks due before next month.
        #[arg(short = 'm', long = "month", conflicts_with_all = &["due", "today", "tomorrow", "week", "year"])]
        month: bool,

        /// Include tasks due before next year.
        #[arg(short = 'y', long = "year", conflicts_with_all = &["due", "today", "tomorrow", "week", "month"])]
        year: bool,

        /// Include only recurring tasks.
        #[arg(short = 'r', long = "recurring")]
        recurring: bool,
    },

    /// Add a new task.
    #[command(name = "add", alias = "a")]
    Add {
        /// Task title/content.
        content: String,

        /// Project name.
        ///
        /// Note: currently recognized but not implemented.
        #[arg(short = 'p', long = "project")]
        project: Option<String>,

        /// Optional task description.
        #[arg(short = 'D', long = "description")]
        description: Option<String>,

        /// Natural-language due date string.
        #[arg(short = 'd', long = "due", conflicts_with_all = &["today", "tomorrow", "week", "month", "year"])]
        due: Option<String>,

        /// Task priority (1-4).
        #[arg(short = 'P', long = "priority")]
        priority: Option<String>,

        /// Set due date to today.
        #[arg(short = 't', long = "today", conflicts_with_all = &["due", "tomorrow", "week", "month", "year"])]
        today: bool,

        /// Set due date to tomorrow.
        #[arg(short = 'T', long = "tomorrow", conflicts_with_all = &["due", "today", "week", "month", "year"])]
        tomorrow: bool,

        /// Set due date to one week from now.
        #[arg(short = 'w', long = "week", conflicts_with_all = &["due", "today", "tomorrow", "month", "year"])]
        week: bool,

        /// Set due date to one month from now.
        #[arg(short = 'm', long = "month", conflicts_with_all = &["due", "today", "tomorrow", "week", "year"])]
        month: bool,

        /// Set due date to one year from now.
        #[arg(short = 'y', long = "year", conflicts_with_all = &["due", "today", "tomorrow", "week", "month"])]
        year: bool,
    },

    /// Complete a task by list index.
    ///
    /// The index comes from the most recent `todo list` output.
    #[command(name = "check", alias = "c")]
    Check { id: usize },

    /// Reopen a completed task by list index.
    ///
    /// The index comes from the most recent `todo list` output.
    #[command(name = "uncheck", alias = "u")]
    Uncheck { id: usize },

    /// Delete a task by list index.
    ///
    /// The index comes from the most recent `todo list` output.
    #[command(name = "delete", alias = "d")]
    Delete { id: usize },
    // #[command(name = "projects", alias = "p")]
    // Projects {},
}

fn auth_headers() -> Result<HeaderMap> {
    let token = sync::get_token()
        .map_err(|e| anyhow!("No Auth Token set: {e}\nPlease set it using `todo auth <token>`"))?;

    let mut headers = HeaderMap::new();
    headers.insert("Authorization", format!("Bearer {token}").parse().unwrap());

    Ok(headers)
}

fn resolve_task_id(id: usize) -> Result<String> {
    let list = sync::get_list("task_ids.txt").map_err(|e| {
        anyhow!("No task list found: {e}\nPlease run `todo list` first to generate the task list.")
    })?;

    list.get(id).cloned().ok_or_else(|| {
        anyhow!(
            "Task ID not found in list: {id}\nPlease ensure you are using a valid task ID from the most recent `todo list` output."
        )
    })
}

fn update_task(id: usize, method: Method, action_path: &str, success_message: &str) -> Result<()> {
    let headers = match auth_headers() {
        Ok(headers) => headers,
        Err(error) => {
            return Err(anyhow!("{error}"));
        }
    };

    let task_id = match resolve_task_id(id) {
        Ok(task_id) => task_id,
        Err(error) => {
            return Err(anyhow!("{error}"));
        }
    };

    let api_link = format!("https://api.todoist.com/api/v1/tasks/{task_id}{action_path}");

    let _body = Client::new()
        .request(method, api_link)
        .headers(headers)
        .send()
        .unwrap();

    println!("{success_message}");
    Ok(())
}

fn validate_response(body: &Response) -> Result<()> {
    let status = body.status();
    if !status.is_success() {
        return Err(anyhow!("API request failed with status: {status}"));
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Commands::Auth { token } => {
            if let Err(e) = sync::save_token(&token) {
                return Err(anyhow!("Error saving token: {e}"));
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
                    return Err(anyhow!("{error}"));
                }
            };

            // parse inputs
            let limit = limit.unwrap_or(25);
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

            let response = Client::new()
                .get(url.clone())
                .headers(headers.clone())
                .send()
                .unwrap();

            match validate_response(&response) {
                Ok(()) => {}
                Err(e) => {
                    return Err(anyhow!("{e}"));
                }
            }

            let body = response.json::<Value>().unwrap();

            // dbg!(&body);

            if let Some(error) = body["error"].as_str() {
                return Err(anyhow!("API Error: {error}"));
            }

            let today = chrono::Local::now().date_naive();
            let fallback_due = chrono::NaiveDate::from_ymd_opt(2222, 1, 1).unwrap();

            let mut ordered = Vec::new();
            let empty_tasks: &[Value] = &[];
            let tasks = body["results"]
                .as_array()
                .map_or(empty_tasks, Vec::as_slice);

            println!("{:<4}| {:<75} | Due Date", "ID", "Content - Description");
            println!("{}", "-".repeat(100));

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

                let line = format!("{content_description:<75} | {due_line:<12}");

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
                    return Err(anyhow!("{error}"));
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

            let response = Client::new().post(url).headers(headers).send().unwrap();

            match validate_response(&response) {
                Ok(()) => {}
                Err(e) => {
                    return Err(anyhow!("API Error: {e}"));
                }
            }

            let body = response.json::<Value>().unwrap();

            // dbg!(&body);

            if let Some(error) = body["error"].as_str() {
                return Err(anyhow!("API Error: {error}"));
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
            sync::save_list(&list, "task_ids.txt")?;
        }

        Commands::Check { id } => {
            update_task(id, Method::POST, "/close", "Task closed successfully.")?;
        }

        Commands::Uncheck { id } => {
            update_task(id, Method::POST, "/reopen", "Task reopened successfully.")?;
        }

        Commands::Delete { id } => {
            update_task(id, Method::DELETE, "", "Task deleted successfully.")?;
        } // Commands::Projects {} => {
          //     todo!("Project listing not implemented yet.");
          // }
    }

    Ok(())
}
