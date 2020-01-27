CREATE TABLE relation (
       id INTEGER PRIMARY KEY NOT NULL,
       from_ident VARCHAR NOT NULL,
       to_ident VARCHAR NOT NULL,
       kind VARCHAR NOT NULL,
       when_created REAL NOT NULL
);
CREATE UNIQUE INDEX relation_from ON relation (kind, from_ident, to_ident);
CREATE UNIQUE INDEX relation_to ON relation (kind, to_ident, from_ident);
CREATE UNIQUE INDEX relation_ident_from ON relation (from_ident, kind, to_ident);
CREATE UNIQUE INDEX relation_ident_to ON relation (to_ident, kind, from_ident);

CREATE VIEW relation_closure AS
       WITH RECURSIVE
            rel_cl(from_ident, to_ident, kind) AS (
                               SELECT from_ident, to_ident, kind
                               FROM relation
                               UNION ALL
                               SELECT rel_cl.from_ident, relation.to_ident, rel_cl.kind
                               FROM rel_cl, relation
                               WHERE rel_cl.kind = relation.kind
                               AND rel_cl.to_ident = relation.from_ident
            )
            SELECT from_ident, to_ident, kind
            FROM rel_cl;

-- CREATE TABLE relation_closure_a AS SELECT * FROM relation_closure;
-- SELECT sql FROM sqlite_master WHERE name = 'relation_closure_a';

CREATE TABLE relation_closure_b(
  id INTEGER PRIMARY KEY NOT NULL,
  from_ident VARCHAR NOT NULL,
  to_ident  VARCHAR NOT NULL,
  kind VARCHAR NOT NULL
);

-- DROP TABLE relation_closure_a;
-- diesel print-schema


