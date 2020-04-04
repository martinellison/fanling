CREATE TABLE item (
       ident VARCHAR NOT NULL PRIMARY KEY,
       type_name VARCHAR NOT NULL,
       name VARCHAR NOT NULL,
       open BOOLEAN NOT NULL DEFAULT(1),
       parent VARCHAR DEFAULT NULL,
       sort VARCHAR NOT NULL,
       classify VARCHAR NOT NULL,
       special INTEGER NOT NULL,
       targeted BOOLEAN NOT NULL DEFAULT(0)
     );
CREATE UNIQUE INDEX item_open ON item (open, sort, ident) WHERE open;
CREATE INDEX item_classify ON item (classify, open, name) WHERE classify != 'normal';
CREATE INDEX item_special ON item (special, open, name) WHERE special != 0;
CREATE INDEX item_child ON item (parent, open);

CREATE VIEW item_by_level AS
       WITH RECURSIVE
            anc(ident2, level, hier_sort) AS (
                        SELECT ident AS ident2, 0, sort  || "!" || ident
                        FROM item
                        WHERE parent IS NULL
                        UNION ALL
                        SELECT item.ident AS ident2, level+1, anc.hier_sort || "!" || item.sort || "!" || item.ident
                        FROM anc, item
                        WHERE anc.ident2 = item.parent
            )
            SELECT *
            FROM anc, item
            WHERE anc.ident2 = item.ident
            ORDER BY anc.hier_sort;

-- need to do the following in sqlite3
-- CREATE TABLE item_by_level_copy AS SELECT * FROM item_by_level;
-- SELECT sql FROM sqlite_master WHERE name = "item_by_level_copy";
-- edit the SQL to make ident the primary key and change the table name to item_by_level_copy2
-- then run the SQL to create the new table
-- then DROP TABLE item_by_level_copy
-- paste the new table in here
-- diesel database reset to rebuild the database with the new table
-- then run `diesel print-schema`
-- then rename the new table back to item_by_level in the diesel schema
CREATE TABLE item_by_level_copy2(
 ident2 varchar NOT NULL,
 level INTEGER NOT NULL,
 hier_sort VARCHAR NOT NULL,
 ident VARCHAR PRIMARY KEY NOT NULL,
 type_name VARCHAR NOT NULL,
 name VARCHAR NOT NULL,
 open BOOLEAN NOT NULL,
 parent VARCHAR,
 sort VARCHAR NOT NULL,
 classify VARCHAR NOT NULL,
 special  INTEGER NOT NULL,
 targeted BOOLEAN NOT NULL
);
