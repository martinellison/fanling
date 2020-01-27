/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! define searches for [`Item`]s */
use crate::fanling_trace;
use crate::item::{Item, ItemListEntryList, ItemRef, SpecialKind};
mod models;
use crate::shared::{FLResult, NullResult, Tracer};
use diesel::prelude::*;
mod schema;
//pub use models::global_row::{read_global, update_last_ident};
use chrono::Utc;
use log::trace;
use std::ops::Deref;
use std::path::Path;

use crate::search::schema::item::dsl;
embed_migrations!("../migrations");
/** a  search for [`Item`]s */
pub struct Search {
    connect: SqliteConnection,
    //    database_path: String,
}
impl Search {
    /** create a new [Search]  */
    pub fn new_and_open(opts: &SearchOptions) -> FLResult<(Self, bool)> {
        let path = opts.database_path.clone();
        let exists = Path::new(&path).exists();
        fanling_trace!(&format!(
            "opening search, search database {:?} {}",
            &path,
            if exists {
                "already exists"
            } else {
                "does not exist"
            }
        ));
        let conn = Self {
            connect: SqliteConnection::establish(&path)?,
        };
        trace(&format!("have sqlite connection, running migrations..."));
        embedded_migrations::run_with_output(&conn.connect, &mut std::io::stdout())?;
        trace("connecting function");
        models::connect_function(&conn.connect);
        trace(&format!(
            "search open ({} entries).",
            conn.search_all()?.entries.len()
        ));
        Ok((conn, !exists))
    }
    /** delete all items from the search database */
    pub fn clear(&mut self) -> NullResult {
        trace("clearing database...");
        Ok(models::delete_all(&self.connect)?)
    }
    /** add an [`Item`] as findable by search */
    pub fn add_item(&mut self, item: &ItemRef) -> NullResult {
        let mut itemx = item.deref().borrow_mut();
        let ident = itemx.ident();
        trace(&format!(
            "adding item '{:?}' to search: {:#?}",
            &ident, &itemx
        ));
        let par_id = itemx.parent_ident();
        let mut ss: String = "".to_owned();
        if let Some(s) = par_id.clone() {
            ss = s.clone();
        }
        let parent = match &par_id {
            None => None,
            Some(_) => Some(ss.as_str()),
        };
        let _naive_date_time = Utc::now().naive_utc();
        Ok(models::create_item(
            &self.connect,
            &models::NewItem {
                ident: &ident,
                type_name: &itemx.type_name(),
                name: &itemx.description(),
                open: itemx.is_open(),
                ready: itemx.is_ready(),
                parent,
                sort: &itemx.get_sort(),
                classify: itemx.classify(),
                special: itemx.specials().val().into(),
                targeted: itemx.targeted(),
            },
        )?)
    }
    /** remove an [`Item`] from the set of searchable values */
    pub fn delete_item(&mut self, item: ItemRef) -> NullResult {
        let itemx = item.borrow();
        let ident = itemx.ident();
        let num_deleted = diesel::delete(dsl::item.find(&ident)).execute(&self.connect)?;
        assert_eq!(1, num_deleted);
        trace(&format!("deleted item '{:?}' from search", &ident));
        Ok(())
    }
    /** item has been modified, do what is necessary */
    pub fn update_item(&mut self, itemx: &mut Item) -> NullResult {
        //    let mut itemx = item_ref.deref().borrow_mut();
        let ident = itemx.ident();
        trace(&format!("updating item in search to {:?}", itemx));
        let _naive_date_time = Utc::now().naive_utc();
        diesel::update(dsl::item.find(&ident))
            .set((
                dsl::name.eq(itemx.description()),
                dsl::open.eq(itemx.is_open()),
                dsl::ready.eq(itemx.is_ready()),
                dsl::parent.eq(itemx.parent_ident()),
                dsl::sort.eq(itemx.get_sort()),
                dsl::classify.eq(itemx.classify()),
                dsl::special.eq(itemx.specials().val() as i32),
                dsl::targeted.eq(itemx.targeted()),
            ))
            .execute(&self.connect)?;
        trace(&format!("updated item '{:?}' in search", &ident));
        Ok(())
    }
    /** search everything  */
    pub fn search_all(&self) -> FLResult<ItemListEntryList> {
        let results = models::search_all(&self.connect)?;
        let iter = ItemListEntryList {
            entries: results.entries,
            final_adjust_level: "".to_owned(),
        };
        Ok(iter)
    }
    // /** search everything for ready */
    // pub fn search_ready(&self) -> FLResult<ItemListEntryList> {
    //     let results = models::search_ready(&self.connect)?;
    //     let iter = ItemListEntryList {
    //         entries: results.entries,
    //         final_adjust_level: "".to_owned(),
    //     };
    //     Ok(iter)
    // }
    /** search everything for parents */
    pub fn search_special(&self, sk: SpecialKind) -> FLResult<ItemListEntryList> {
        let results = models::search_special(&self.connect, sk)?;
        let iter = ItemListEntryList {
            entries: results.entries,
            final_adjust_level: "".to_owned(),
        };
        Ok(iter)
    }
    /** search for children */
    pub fn search_ready_children(&self, parent_ident: &str) -> FLResult<ItemListEntryList> {
        let results = models::search_ready_children(&self.connect, parent_ident)?;
        let iter = ItemListEntryList {
            entries: results.entries,
            final_adjust_level: "".to_owned(),
        };
        Ok(iter)
    }
    /** search everything for ready with hierarchy */
    pub fn search_ready_hier(&self) -> FLResult<ItemListEntryList> {
        let results = models::search_ready_hier(&self.connect)?;
        trace(&format!("hier found {} entries", results.entries.len()));
        let iter = ItemListEntryList {
            entries: results.entries,
            final_adjust_level: "".to_owned(),
        };
        Ok(iter)
    }
    /** find the global row */
    pub fn read_global(&self) -> FLResult<(i32, String)> {
        models::global_row::read_global(&self.connect)
    }
    /** update the global last ident */
    pub fn update_last_ident(&mut self, last_ident: i32) -> NullResult {
        models::global_row::update_last_ident(&mut self.connect, last_ident)
    }
    // /** search relative to a specific [`Item`] */
    // pub fn search_relative(&self, _item: ItemRef) -> ItemListEntryList {
    //     unimplemented!(); //
    // }
}
impl Drop for Search {
    fn drop(&mut self) {
        trace("dropping search");
    }
}

/** options for a [Search]. */
#[derive(Debug)]
pub struct SearchOptions {
    pub database_path: String,
}
/** convenience function for debug traces */
fn trace(txt: &str) {
    trace!("{}", txt);
    println!(
        "search {}",
        ansi_term::Colour::Purple
            .on(ansi_term::Colour::Cyan)
            .paint(txt)
    );
}
