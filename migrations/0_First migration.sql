-- public.reminders definition

CREATE TABLE public.reminders (
                                  who varchar NOT NULL,
                                  "when" timestamptz NOT NULL,
                                  what varchar NOT NULL,
                                  "server" varchar NOT NULL,
                                  channel varchar NOT NULL,
                                  id bigint NOT NULL GENERATED ALWAYS AS IDENTITY
);

CREATE INDEX reminders_when_idx ON public.reminders ("when");