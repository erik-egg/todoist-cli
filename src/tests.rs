use chrono::{Duration, Local, NaiveDate};

use crate::{build_filter, format_due_date, parse_due_date};

#[test]
fn parse_due_date_parses_datetime() {
    let parsed = parse_due_date("2026-03-10T14:45:59Z");

    assert_eq!(
        parsed,
        Some(
            NaiveDate::from_ymd_opt(2026, 3, 10)
                .unwrap()
                .and_hms_opt(14, 45, 59)
                .unwrap()
        )
    );
}

#[test]
fn parse_due_date_parses_date_only_to_midnight() {
    let parsed = parse_due_date("2026-03-10");

    assert_eq!(
        parsed,
        Some(
            NaiveDate::from_ymd_opt(2026, 3, 10)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        )
    );
}

#[test]
fn parse_due_date_returns_none_for_invalid_input() {
    let parsed = parse_due_date("not-a-date");

    assert_eq!(parsed, None);
}

#[test]
fn format_due_date_returns_empty_for_none() {
    assert_eq!(format_due_date(None).to_string(), "");
}

#[test]
fn date_to_display_handles_yesterday_with_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_sub_signed(Duration::days(1))
        .unwrap()
        .and_hms_opt(9, 30, 0)
        .unwrap();

    assert_eq!(format_due_date(Some(date)).to_string(), "Yesterday 09:30");
}

#[test]
fn date_to_display_handles_yesterday_without_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_sub_signed(Duration::days(1))
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    assert_eq!(format_due_date(Some(date)).to_string(), "Yesterday");
}

#[test]
fn date_to_display_handles_today_with_time() {
    let today = Local::now().date_naive();
    let date = today.and_hms_opt(16, 20, 0).unwrap();

    assert_eq!(format_due_date(Some(date)).to_string(), "Today 16:20");
}

#[test]
fn date_to_display_handles_today_without_time() {
    let today = Local::now().date_naive();
    let date = today.and_hms_opt(0, 0, 0).unwrap();

    assert_eq!(format_due_date(Some(date)).to_string(), "Today");
}

#[test]
fn date_to_display_handles_tomorrow_with_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_add_signed(Duration::days(1))
        .unwrap()
        .and_hms_opt(7, 5, 0)
        .unwrap();

    assert_eq!(format_due_date(Some(date)).to_string(), "Tomorrow 07:05");
}

#[test]
fn date_to_display_handles_tomorrow_without_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_add_signed(Duration::days(1))
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    assert_eq!(format_due_date(Some(date)).to_string(), "Tomorrow");
}

#[test]
fn date_to_display_handles_within_next_week_with_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_add_signed(Duration::days(5))
        .unwrap()
        .and_hms_opt(11, 40, 0)
        .unwrap();

    assert_eq!(
        format_due_date(Some(date)).to_string(),
        date.format("%A %H:%M").to_string()
    );
}

#[test]
fn date_to_display_handles_within_next_week_without_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_add_signed(Duration::days(6))
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    assert_eq!(
        format_due_date(Some(date)).to_string(),
        date.format("%A").to_string()
    );
}

#[test]
fn date_to_display_defaults_for_other_dates_with_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_sub_signed(Duration::days(2))
        .unwrap()
        .and_hms_opt(22, 10, 0)
        .unwrap();

    assert_eq!(
        format_due_date(Some(date)).to_string(),
        date.format("%Y-%m-%d %H:%M").to_string()
    );
}

#[test]
fn date_to_display_defaults_for_other_dates_without_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_add_signed(Duration::days(8))
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    assert_eq!(
        format_due_date(Some(date)).to_string(),
        date.format("%Y-%m-%d").to_string()
    );
}

#[test]
fn build_filter_prefers_raw_filter_when_present() {
    let filter = build_filter(
        Some(String::from("raw filter")),
        Some(String::from("search-term")),
        Some(String::from("Inbox")),
        Some(String::from("today")),
        Some(String::from("tomorrow")),
        Some(String::from("next week")),
        Some(String::from("1")),
        true,
        true,
        true,
        true,
        true,
        true,
        true,
    );

    assert_eq!(filter, Some(String::from("raw filter")));
}

#[test]
fn build_filter_returns_none_when_no_components_are_provided() {
    let filter = build_filter(
        None, None, None, None, None, None, None, false, false, false, false, false, false, false,
    );

    assert_eq!(filter, None);
}

#[test]
fn build_filter_combines_components_in_expected_order() {
    let filter = build_filter(
        None,
        Some(String::from("feature")),
        Some(String::from("Work")),
        Some(String::from("today")),
        Some(String::from("tomorrow")),
        Some(String::from("next week")),
        Some(String::from("2")),
        true,
        true,
        true,
        true,
        true,
        true,
        true,
    );

    assert_eq!(
        filter,
        Some(String::from(
            "search: feature & ##Work & due: today & due before: tomorrow & due after: next week & p2 & overdue & due: today & due: tomorrow & due before: next week & due before: next month & due before: next year & recurring"
        ))
    );
}
