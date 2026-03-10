use chrono::{Duration, Local, NaiveDate};

use crate::{opt_date_to_display, string_to_date};

#[test]
fn string_to_date_parses_datetime() {
    let parsed = string_to_date("2026-03-10T14:45:59Z");

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
fn string_to_date_parses_date_only_to_midnight() {
    let parsed = string_to_date("2026-03-10");

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
fn string_to_date_returns_none_for_invalid_input() {
    let parsed = string_to_date("not-a-date");

    assert_eq!(parsed, None);
}

#[test]
fn opt_date_to_display_returns_empty_for_none() {
    assert_eq!(opt_date_to_display(None).to_string(), "");
}

#[test]
fn date_to_display_handles_yesterday_with_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_sub_signed(Duration::days(1))
        .unwrap()
        .and_hms_opt(9, 30, 0)
        .unwrap();

    assert_eq!(
        opt_date_to_display(Some(date)).to_string(),
        "Yesterday 09:30"
    );
}

#[test]
fn date_to_display_handles_yesterday_without_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_sub_signed(Duration::days(1))
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    assert_eq!(opt_date_to_display(Some(date)).to_string(), "Yesterday");
}

#[test]
fn date_to_display_handles_today_with_time() {
    let today = Local::now().date_naive();
    let date = today.and_hms_opt(16, 20, 0).unwrap();

    assert_eq!(opt_date_to_display(Some(date)).to_string(), "Today 16:20");
}

#[test]
fn date_to_display_handles_today_without_time() {
    let today = Local::now().date_naive();
    let date = today.and_hms_opt(0, 0, 0).unwrap();

    assert_eq!(opt_date_to_display(Some(date)).to_string(), "Today");
}

#[test]
fn date_to_display_handles_tomorrow_with_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_add_signed(Duration::days(1))
        .unwrap()
        .and_hms_opt(7, 5, 0)
        .unwrap();

    assert_eq!(
        opt_date_to_display(Some(date)).to_string(),
        "Tomorrow 07:05"
    );
}

#[test]
fn date_to_display_handles_tomorrow_without_time() {
    let today = Local::now().date_naive();
    let date = today
        .checked_add_signed(Duration::days(1))
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    assert_eq!(opt_date_to_display(Some(date)).to_string(), "Tomorrow");
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
        opt_date_to_display(Some(date)).to_string(),
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
        opt_date_to_display(Some(date)).to_string(),
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
        opt_date_to_display(Some(date)).to_string(),
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
        opt_date_to_display(Some(date)).to_string(),
        date.format("%Y-%m-%d").to_string()
    );
}
