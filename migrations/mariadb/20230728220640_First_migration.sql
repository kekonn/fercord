-- fercord.reminders definition

CREATE TABLE `reminders` (
  `id` uuid NOT NULL DEFAULT uuid(),
  `who` varchar(100) NOT NULL,
  `what` varchar(1000) NOT NULL,
  `server` varchar(100) NOT NULL,
  `channel` varchar(100) NOT NULL,
  `when` datetime NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

ALTER TABLE fercord.reminders ADD CONSTRAINT reminders_PK PRIMARY KEY (id);
