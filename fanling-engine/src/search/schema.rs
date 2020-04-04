table! {
    global (id) {
        id -> Integer,
        ident_prefix -> Text,
        last_ident -> Integer,
    }
}

table! {
    item (ident) {
        ident -> Text,
        type_name -> Text,
        name -> Text,
        open -> Bool,
        parent -> Nullable<Text>,
        sort -> Text,
        classify -> Text,
        special -> Integer,
        targeted -> Bool,
    }
}

table! {
    item_by_level (ident) {
        ident2 -> Text,
        level -> Integer,
        hier_sort -> Text,
        ident -> Text,
        type_name -> Text,
        name -> Text,
        open -> Bool,
        parent -> Nullable<Text>,
        sort -> Text,
        classify -> Text,
        special -> Integer,
        targeted -> Bool,
    }
}

table! {
    relation (id) {
        id -> Integer,
        from_ident -> Text,
        to_ident -> Text,
        kind -> Text,
        when_created -> Float,
    }
}

table! {
    relation_closure (id) {
        id -> Integer,
        from_ident -> Text,
        to_ident -> Text,
        kind -> Text,
    }
}

table! {
    task (ident) {
        ident -> Text,
        show_after -> Float,
        deadline -> Nullable<Float>,
        when_closed -> Nullable<Float>,
        context -> Text,
        priority -> Integer,
        status -> Text,
        blocked -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    global,
    item,
    item_by_level,
    relation,
    relation_closure,
    task,
);
