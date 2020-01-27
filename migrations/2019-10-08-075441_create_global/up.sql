CREATE TABLE global (
       id INTEGER NOT NULL DEFAULT(0) PRIMARY KEY,
       ident_prefix VARCHAR NOT NULL,
       last_ident INTEGER NOT NULL DEFAULT(0)
);
INSERT INTO global VALUES (0, "@", 0);
