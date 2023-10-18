
-- Create reminders table

CREATE TABLE reminders (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	who TEXT(128),
	"when" TEXT(128),
	what TEXT(1024),
	server TEXT(128),
	channel TEXT(128)
);