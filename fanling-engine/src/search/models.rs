/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! database code */
use crate::item::{ItemLinkForSerde, ItemListEntry, ItemListEntryList, SpecialKind, SpecialKinds};
pub use crate::search::schema::{global, item, item_by_level};
use crate::shared::{FLResult, NullResult};
use bitfield::Bit;
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer};
use std::convert::TryInto;

// Item table

#[derive(Queryable)]
/** an [Item]  in the database */
pub struct DslItem {
    ident: String,
    _type_name: String,
    name: String,
    _open: bool,
    _ready: bool,
    _parent: Option<String>,
    _sort: String,
    _classify: String,
    _special: i32,
    _targeted: bool,
}
impl Into<ItemListEntry> for DslItem {
    fn into(self) -> ItemListEntry {
        ItemListEntry {
            link: ItemLinkForSerde::new(self.ident.clone()),
            descr: self.name,
            ..ItemListEntry::default()
        }
    }
}

#[derive(Insertable)]
#[table_name = "item"]
/** a new [Item] in the database for inserts */
pub struct NewItem<'a> {
    pub ident: &'a str,
    pub type_name: &'a str,
    pub name: &'a str,
    pub open: bool,
    pub ready: bool,
    pub parent: Option<&'a str>,
    pub sort: &'a str,
    pub classify: String,
    pub special: i32,
    pub targeted: bool,
}

/** create a new item in the database */
pub fn create_item<'a>(conn: &SqliteConnection, new_item: &NewItem) -> NullResult {
    Ok(diesel::insert_into(item::table)
        .values(new_item)
        .execute(conn)
        .map(|_n| ())?)
}
/** delete all items in the database */
pub fn delete_all(conn: &SqliteConnection) -> NullResult {
    let num_deleted = diesel::delete(item::table).execute(conn)?;
    trace(&format!("{} items deleted from search", num_deleted));
    Ok(())
}
/** find all items in the databas */
pub fn search_all(conn: &SqliteConnection) -> FLResult<ItemListEntryList> {
    let results = item::dsl::item.load::<DslItem>(conn)?;
    Ok(ItemListEntryList::from_vec(
        results.into_iter().map(DslItem::into).collect(),
    ))
}
sql_function!(fn has_special(skk: Integer, sk: Integer)->Bool);
/** connect up an SQL function `has_special` */
pub fn connect_function(conn: &SqliteConnection) {
    has_special::register_impl(&conn, |skk: i32, sk: i32| {
        let mut msk = SpecialKinds::default();
        let skx: usize = sk.try_into().expect("bad???");
        msk.set_bit(skx, true);
        (skk & (msk.val() as i32)) != 0
    })
    .expect("bad???");
}

// /** find all ready items in the database */
// pub fn search_ready(conn: &SqliteConnection) -> FLResult<ItemListEntryList> {
//     connect_function(conn); // TODO: only once in new
//     let results = item::dsl::item
//         .filter(item::columns::ready.eq(true))
//         .load::<DslItem>(conn)?;
//     Ok(ItemListEntryList::from_vec(
//         results.into_iter().map(DslItem::into).collect(),
//     ))
// }
/** find all potential parents in the database */
pub fn search_special(conn: &SqliteConnection, sk: SpecialKind) -> FLResult<ItemListEntryList> {
    //    let skk = sk. as_bitmap();
    let results = item::dsl::item
        .filter(
            item::columns::open
                .eq(true)
                .and(has_special(item::columns::special, sk as i32)),
        )
        .load::<DslItem>(conn)?;
    Ok(ItemListEntryList::from_vec(
        results.into_iter().map(DslItem::into).collect(),
    ))
}
/** */
pub fn search_ready_children(
    conn: &SqliteConnection,
    parent_ident: &str,
) -> FLResult<ItemListEntryList> {
    let results = item::dsl::item
        .filter(item::columns::ready.and(item::columns::parent.eq(parent_ident)))
        .load::<DslItem>(conn)?;
    let ilev: Vec<ItemListEntry> = results.into_iter().map(DslItem::into).collect();
    Ok(ItemListEntryList::from_vec(ilev))
}

// Item hierarchy

#[derive(Queryable)]
/** results of a hierarchy query */
pub struct DslItemHier {
    _ident2: String,
    level: i32,
    _hier_sort: String,
    ident: String,
    _type_name: String,
    name: String,
    _open: bool,
    _ready: bool,
    _parent: Option<String>,
    _sort: String,
    _classify: String,
    _special: i32,
    _targeted: bool,
}
impl Into<ItemListEntry> for DslItemHier {
    fn into(self) -> ItemListEntry {
        ItemListEntry {
            link: ItemLinkForSerde::new(self.ident.clone()),
            descr: self.name.clone(),
            level: self.level as i8,
            ..ItemListEntry::default()
        }
    }
}

/** find all ready items in the database in hierarchy */
pub fn search_ready_hier(conn: &SqliteConnection) -> FLResult<ItemListEntryList> {
    let results = item_by_level::dsl::item_by_level
        .filter(item_by_level::columns::open)
        .load::<DslItemHier>(conn)?;
    let ilev: Vec<ItemListEntry> = results.into_iter().map(DslItemHier::into).collect();
    Ok(ItemListEntryList::from_vec(ilev))
}

pub mod global_row {
    use crate::search::schema::global;
    use crate::shared::{FLResult, NullResult};
    use diesel::prelude::*;
    // Global table

    #[derive(Queryable)]
    /** the [Global]  in the database. This table only has one row. */
    pub struct DslGlobal {
        _id: i32,
        ident_prefix: String,
        last_ident: i32,
    }
    /** find the global row */
    pub fn read_global(conn: &SqliteConnection) -> FLResult<(i32, String)> {
        let result = global::dsl::global.get_result::<DslGlobal>(conn)?;
        Ok((result.last_ident, result.ident_prefix))
    }
    /** update the global last ident */
    pub fn update_last_ident(conn: &mut SqliteConnection, last_ident: i32) -> NullResult {
        diesel::update(global::dsl::global)
            .set(global::dsl::last_ident.eq(last_ident))
            .execute(conn)?;
        Ok(())
    }
}
// helper

/** convenience function for debug traces */
fn trace(txt: &str) {
    println!(
        "model {}",
        ansi_term::Colour::White
            .on(ansi_term::Colour::Yellow)
            .paint(txt)
    );
}
