CREATE TABLE task (
       ident VARCHAR NOT NULL PRIMARY KEY,
       show_after REAL NOT NULL,
       deadline REAL,
       when_closed REAL,
       context VARCHAR NOT NULL,
       priority INTEGER NOT NULL,
       status VARCHAR NOT NULL,
       blocked BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX task_context ON task (context);
CREATE INDEX task_status ON task (status);
