-- public.reminders definition

CREATE TABLE public.reminders (
                                  who numeric NOT NULL,
                                  "when" timestamptz NOT NULL,
                                  what varchar NOT NULL,
                                  "server" numeric NOT NULL,
                                  channel numeric NOT NULL,
                                  id bigint NOT NULL GENERATED ALWAYS AS IDENTITY
);
CREATE INDEX reminders_when_idx ON public.reminders ("when");