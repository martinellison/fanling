#!/usr/bin/env bash
echo "recreating database and getting (and hacking) database schema..."
SCHEMA=$BASE/fanling-engine/src/search/schema.rs
diesel print-schema >$SCHEMA
sed -i -e "s/item_by_level_copy2/item_by_level/g" $SCHEMA
sed -i -e "s/relation_closure_b/relation_closure/g" $SCHEMA
