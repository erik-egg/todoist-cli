mod sync;

#[cfg(test)]
mod tests;

use anyhow::{Result, anyhow};
use chrono::NaiveDateTime;
use clap::{Parser, Subcommand};
use colored::{ColoredString, Colorize};
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

        #[arg(short = 'q', long = "quiet", default_value_t = false)]
        /// Suppress output after saving the token.
        quiet: bool,
    },

    /// List tasks with optional filters.
    #[command(name = "list", alias = "l", alias = "get", alias = "g")]
    List {
        /// Raw Todoist filter string like outlined here: `<https://www.todoist.com/help/articles/introduction-to-filters-V98wIH>`
        ///
        /// Alternatively you can also use the more user-friendly filter components below that will be combined into a filter string with AND logic.
        #[arg(conflicts_with_all = &["search", "project", "due", "before", "after", "overdue", "today", "tomorrow", "week", "recurring"])]
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
        /// The text of the task that is parsed.
        ///
        /// It can include:
        /// - a due date in free form text
        /// - a project name starting with the # character (without spaces)
        /// - a label starting with the @ character
        /// - an assignee starting with the + character
        /// - a priority (e.g., p1)
        /// - a deadline between {} (e.g. {in 3 days})
        /// - a description starting from // until the end of the text.
        content: String,

        /// A natural language reminder to set for the task.
        #[arg(short = 'r', long = "reminder")]
        reminder: Option<String>,
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
    headers.insert("Authorization", format!("Bearer {token}").parse()?);

    Ok(headers)
}

fn resolve_task_id_from_index(list_index: usize) -> Result<String> {
    let task_ids = sync::get_list("task_ids.txt").map_err(|e| {
        anyhow!("No task list found: {e}\nPlease run `todo list` first to generate the task list.")
    })?;

    task_ids.get(list_index).cloned().ok_or_else(|| {
        anyhow!(
            "Task ID not found in list index: {list_index}\nPlease ensure you are using a valid task ID from the most recent `todo list` output."
        )
    })
}

fn update_task_by_index(
    list_index: usize,
    method: Method,
    action_suffix: &str,
    success_message: &str,
) -> Result<()> {
    let auth_header_map = match auth_headers() {
        Ok(headers) => headers,
        Err(error) => {
            return Err(anyhow!("{error}"));
        }
    };

    let task_id = match resolve_task_id_from_index(list_index) {
        Ok(task_id) => task_id,
        Err(error) => {
            return Err(anyhow!("{error}"));
        }
    };

    let api_url = format!("https://api.todoist.com/api/v1/tasks/{task_id}{action_suffix}");

    Client::new()
        .request(method, api_url)
        .headers(auth_header_map)
        .send()?;

    println!("{success_message}");
    Ok(())
}

fn validate_response_status(response: &Response) -> Result<()> {
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("API request failed with status: {status}"));
    }
    Ok(())
}

fn parse_due_date(date_str: &str) -> Option<chrono::NaiveDateTime> {
    // If the string includes time, parse it as NaiveDateTime
    if let Ok(date_time) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%SZ") {
        return Some(date_time);
    }

    // Alternatively, try parsing with just date (time defaults to 00:00:00)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return date.and_hms_opt(0, 0, 0);
    }

    None
}

fn format_due_date(date: Option<NaiveDateTime>) -> ColoredString {
    let date = match date {
        Some(date) => date,
        None => return ColoredString::default(),
    };

    let today = chrono::Local::now().date_naive();

    // if yesterday, show "Yesterday HH:MM" if time is included, otherwise just "Yesterday"
    if date.date() == today - chrono::Duration::days(1) {
        if date.time() != chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            return date.format("Yesterday %H:%M").to_string().red();
        }
        return "Yesterday".red();
    }

    // if today, show "Today HH:MM" if time is included, otherwise just "Today"
    if date.date() == today {
        if date.time() != chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            return date.format("Today %H:%M").to_string().green();
        }
        return "Today".green();
    }

    // if tomorrow, show "Tomorrow HH:MM" if time is included, otherwise just "Tomorrow"
    if date.date() == today + chrono::Duration::days(1) {
        if date.time() != chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            return date.format("Tomorrow %H:%M").to_string().yellow();
        }
        return "Tomorrow".yellow();
    }

    // if within the next week show "Weekday HH:MM" if time is included, otherwise just "Weekday"
    if date.date() > today && date.date() <= today + chrono::Duration::days(7) {
        if date.time() != chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            return date.format("%A %H:%M").to_string().purple();
        }
        return date.format("%A").to_string().purple();
    }

    // default to "YYYY-MM-DD HH:MM" if time is included, otherwise just "YYYY-MM-DD"
    let formatted_due = if date.time() == chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
        date.format("%Y-%m-%d").to_string()
    } else {
        date.format("%Y-%m-%d %H:%M").to_string()
    };

    if date < today.into() {
        formatted_due.red()
    } else {
        formatted_due.normal()
    }
}

fn display_sorted_tasks(body: &Value) -> Result<()> {
    let mut sorted_tasks = Vec::new();

    let empty_results: &[Value] = &[];
    let task_results = body["results"]
        .as_array()
        .map_or(empty_results, Vec::as_slice);

    let today_at_midnight = chrono::Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    // use a far future date for tasks without a due date to sort them at the end of the list
    let fallback_due_datetime = chrono::NaiveDate::from_ymd_opt(22222, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    for task in task_results {
        // output line construction
        let content = task["content"].as_str().unwrap_or_default();
        let description = task["description"].as_str().unwrap_or_default();
        let task_due = task["due"]["date"].as_str().unwrap_or("none");
        let is_recurring = task["due"]["is_recurring"].as_bool().unwrap_or(false);

        let mut content_with_description = content.to_owned();

        if !description.is_empty() {
            if !content_with_description.is_empty() {
                content_with_description.push_str(" - ");
            }
            content_with_description.push_str(description);
        }

        let parsed_due = parse_due_date(task_due);

        let mut due_line = format_due_date(parsed_due);

        let mut date_for_ordering = parsed_due.unwrap_or(fallback_due_datetime);
        if date_for_ordering.time() == chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            // if no time is included, use the end of the day for ordering to avoid putting tasks without time at the top of the list
            date_for_ordering += chrono::Duration::hours(23)
                + chrono::Duration::minutes(59)
                + chrono::Duration::seconds(59);
        }

        if date_for_ordering < today_at_midnight {
            due_line.input.push('⏰');
        }

        if is_recurring {
            due_line.input.push('🔁');
        }

        let formatted_row = format!("{content_with_description:<75} | {due_line:<12}");

        sorted_tasks.push((
            date_for_ordering,
            formatted_row,
            task["id"].as_str().unwrap_or_default().to_owned(),
        ));
    }

    sorted_tasks.sort_by_key(|&(date, _, _)| date);
    for (idx, (_, row, _)) in sorted_tasks.iter().enumerate() {
        println!("{idx:<4}| {row}");
    }

    let task_ids = sorted_tasks
        .into_iter()
        .map(|(_, _, id)| id)
        .collect::<Vec<String>>();

    sync::save_list(&task_ids, "task_ids.txt")?;
    Ok(())
}

fn build_filter(
    filter: Option<String>,
    search: Option<String>,
    project: Option<String>,
    due: Option<String>,
    before: Option<String>,
    after: Option<String>,
    priority: Option<String>,
    overdue: bool,
    today: bool,
    tomorrow: bool,
    week: bool,
    month: bool,
    year: bool,
    recurring: bool,
) -> Option<String> {
    filter.or_else(|| {
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
    })
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Commands::Auth { token, quiet } => {
            if let Err(e) = sync::save_token(&token) {
                return Err(anyhow!("Error saving token: {e}"));
            }
            if !quiet {
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
            let auth_header_map = match auth_headers() {
                Ok(headers) => headers,
                Err(error) => {
                    return Err(anyhow!("{error}"));
                }
            };

            // parse input filters
            let task_limit = limit.unwrap_or(25);
            let filter_query = build_filter(
                filter, search, project, due, before, after, priority, overdue, today, tomorrow,
                week, month, year, recurring,
            );

            // construct request
            let api_endpoint = if filter_query.is_none() {
                "https://api.todoist.com/api/v1/tasks"
            } else {
                "https://api.todoist.com/api/v1/tasks/filter"
            };

            let mut url = Url::parse(api_endpoint)?;
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("limit", &task_limit.to_string());

                if let Some(filter_query) = filter_query {
                    query.append_pair("query", &filter_query);
                }
            }

            let http_response = Client::new()
                .get(url.clone())
                .headers(auth_header_map.clone())
                .send()?;

            validate_response_status(&http_response)?;

            let response_body = http_response.json::<Value>()?;

            println!("{:<4}| {:<75} | Due Date", "ID", "Content - Description");
            println!("{}", "-".repeat(100));

            display_sorted_tasks(&response_body)?;
        }

        Commands::Add { content, reminder } => {
            let auth_header_map = match auth_headers() {
                Ok(headers) => headers,
                Err(error) => {
                    return Err(anyhow!("{error}"));
                }
            };

            let api_endpoint = "https://api.todoist.com/api/v1/tasks/quick";

            let mut url = Url::parse(api_endpoint)?;
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("text", &content);
                if let Some(reminder) = reminder {
                    query.append_pair("reminder", &reminder);
                }
            }

            let http_response = Client::new()
                .post(url.clone())
                .headers(auth_header_map.clone())
                .send()?;

            match validate_response_status(&http_response) {
                Ok(()) => {}
                Err(e) => {
                    return Err(anyhow!("API Error: {e}"));
                }
            }

            let response_body = http_response.json::<Value>()?;

            // dbg!(&body);

            if let Some(error) = response_body["error"].as_str() {
                return Err(anyhow!("API Error: {error}"));
            }

            let created_task_content = response_body["content"].as_str().unwrap_or_default();
            let created_task_description =
                response_body["description"].as_str().unwrap_or_default();
            let due_text = response_body["due"]["date"].as_str().unwrap_or("none");
            let is_recurring = response_body["due"]["is_recurring"]
                .as_bool()
                .unwrap_or(false);

            let mut content_with_description = created_task_content.to_owned();
            if !created_task_description.is_empty() {
                if !content_with_description.is_empty() {
                    content_with_description.push_str(" - ");
                }
                content_with_description.push_str(created_task_description);
            }

            let recurring_marker = if is_recurring { " 🔁" } else { "" };
            println!("0  | {content_with_description} (due: {due_text}{recurring_marker})");

            let task_ids = vec![response_body["id"].as_str().unwrap_or_default().to_owned()];
            sync::save_list(&task_ids, "task_ids.txt")?;
        }

        Commands::Check { id } => {
            update_task_by_index(id, Method::POST, "/close", "Task closed successfully.")?;
        }

        Commands::Uncheck { id } => {
            update_task_by_index(id, Method::POST, "/reopen", "Task reopened successfully.")?;
        }

        Commands::Delete { id } => {
            update_task_by_index(id, Method::DELETE, "", "Task deleted successfully.")?;
        } // Commands::Projects {} => {
          //     todo!("Project listing not implemented yet.");
          // }
    }

    Ok(())
}
