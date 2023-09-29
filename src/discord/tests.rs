use chrono::Utc;
use tracing_test::traced_test;

use crate::discord::commands::*;

/// `"5 minutes"`
const EXPECTED: &str = "5 minutes";

#[traced_test]
#[test]
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

#[test]
fn clean_input_cleans_in_syntax() {
    let input = "in 5 minutes";

    let result = clean_input(input.into());

    assert_eq!(EXPECTED, result);
}

#[test]
fn clean_input_cleans_from_syntax() {
    let input = "5 minutes from now";

    let result = clean_input(input.into());

    assert_eq!(EXPECTED, result);
}

#[test]
fn clean_input_cleans_combined_syntax() {
    let input = "in 5 minutes from now";

    let result = clean_input(input.into());

    assert_eq!(EXPECTED, result);
}
