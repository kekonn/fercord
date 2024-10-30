pub(super) const REMINDERS_BETWEEN_QUERY: &str = r#"SELECT *
FROM public.reminders
WHERE "when" >= $1 and "when" < $2
"#;

pub(super) const INSERT_QUERY: &str = r#"INSERT INTO public.reminders
(who, "when", what, "server", channel)
VALUES($1, $2, $3, $4, $5) RETURNING id;"#;

pub(super) const DELETE_QUERY: &str = "DELETE FROM public.reminders WHERE id=$1;";

pub(super) const GET_ONE_QUERY: &str = "SELECT * FROM public.reminders WHERE id = $1";
