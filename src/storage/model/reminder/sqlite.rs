
pub(super) const REMINDERS_SINCE_QUERY: &str = r#"SELECT *
FROM reminders
WHERE 'when' >= ? and 'when' < ?
"#;

pub(super) const INSERT_QUERY: &str = r#"INSERT INTO reminders
(who, 'when', what, server, channel)
VALUES(?, ?, ?, ?, ?) RETURNING id;"#;

pub(super) const DELETE_QUERY: &str = "DELETE FROM reminders WHERE id = ?;";

pub(super) const GET_ONE_QUERY: &str = "SELECT * FROM reminders WHERE id = ?";