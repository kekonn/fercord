use chrono::{Duration, NaiveTime, TimeDelta, Timelike, Utc};
use rstest::*;

use crate::discord::commands::*;

/// `"5 minutes"`
const EXPECTED: &str = "5 minutes";

fn duration_for_dhm_from_now(days: i64, h: u32, m: u32, s: u32) -> TimeDelta {
    let now = Utc::now();
    let time = NaiveTime::from_hms_opt(h, m, s).expect("Expected valid NaiveTime");
    let days_from_now = (now + Duration::days(days)).date_naive();

    days_from_now.and_time(time) - now.naive_utc().with_nanosecond(0).unwrap()
}

#[rstest]
#[case("in 5 minutes", TimeDelta::minutes(5))]
#[case("tomorrow at 8am", duration_for_dhm_from_now(1.into(), 8, 0, 0))]
#[case("in 1 hour", TimeDelta::hours(1))]
fn check_parser(#[case] input: &str, #[case] expected: TimeDelta) {
    // Arrange
    let now = Utc::now().with_nanosecond(0).unwrap();

    // Act
    let parsed = parse_human_time(input, Utc, Some(now));

    // Assert
    assert!(parsed.is_ok(), "Could not parse '{input}' to a DateTime");
    let unwrapped = parsed.unwrap();
    assert!(unwrapped > now, "Parsed time appears to be in the past: unwrapped={unwrapped} now={now}");
    assert_eq!(expected, (unwrapped - now), "Parsed time ({unwrapped}) does not match the expected time ({expected})");
}

#[rstest]
fn parsed_moment_is_future() {
    // Arrange
    let input = "in 5 minutes";
    let now = Utc::now();

    // Act
    let parsed = parse_human_time(input, Utc, None);

    // Assert
    debug_assert!(parsed.is_ok(), "We did not get a successful parse");
    let unwrapped = parsed.unwrap();
    assert!(
        unwrapped > now,
        "Parsed time appears to be in the past: unwrapped={unwrapped} now={now}"
    );

    let difference = unwrapped - now;
    assert_eq!(difference.num_minutes(), 5);
}

#[rstest]
fn clean_input_cleans_in_syntax() {
    let input = "in 5 minutes";

    let result = clean_input(input.into());

    assert_eq!(EXPECTED, result);
}

#[rstest]
fn clean_input_cleans_from_syntax() {
    let input = "5 minutes from now";

    let result = clean_input(input.into());

    assert_eq!(EXPECTED, result);
}

#[rstest]
fn clean_input_cleans_combined_syntax() {
    let input = "in 5 minutes from now";

    let result = clean_input(input.into());

    assert_eq!(EXPECTED, result);
}
