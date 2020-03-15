/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! overall code for mapping idents into items */
use crate::fanling_trace;
use crate::item::Ident;
use crate::item::{Item, ItemBaseForSerde, ItemRef};
use crate::shared::{FLResult, FanlingError, NullResult, Tracer};
use regex::Regex;
use taipo_git_control::MergeOutcome;
use taipo_git_control::{
    Change, ChangeList, ConflictList, EntryDescr, FanlingRepository, ObjectOperation,
    RepoActionRequired, RepoOptions,
};

use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::Deref;
//#[macro_use]
use crate::fanling_error;

/** FUTURE: check that repo "file names" (within the repo) are the same as in fanling9 namely `items/_ident_.page`
*/

/** maps idents into items.

This delegates to the repository as required */
pub struct Store {
    repo: FanlingRepository,
    known: HashMap<Ident, ItemRef>,
    pending_changes: ChangeList,
    unallowed_chars: Regex,
    next_ident_num: u64,
    item_path_re: Regex,
    initial_dash: Regex,
}
impl Store {
    /** create and open a [Store] */
    pub fn new_and_open(opts: &RepoOptions) -> Result<(Store, RepoActionRequired), FanlingError> {
        fanling_trace!("opening store");
        let (repo, repo_action_required) = FanlingRepository::new_open(opts)?;
        trace(&format!("store open ({:?}).", repo_action_required));
        Ok((
            Self {
                repo,
                known: HashMap::new(),
                pending_changes: vec![],
                unallowed_chars: Regex::new("[^0-9a-zA-Z]+")?,
                next_ident_num: 0,
                item_path_re: Regex::new("^([^.]*)[.](item|page)$")?,
                initial_dash: Regex::new("^-")?,
            },
            repo_action_required,
        ))
    }
    // TODO: load at start  see tests::open_existing()
    /** clear known values (invalidates all [`Item`]s) */
    pub fn clear_known(&mut self) {
        self.known.clear();
    }
    /** are there any commits that have not been pushed to the remote? */
    pub fn does_need_pushing(&self) -> bool {
        self.repo.does_need_pushing()
    }
    /** add an [`Item`] to the store */
    pub fn add_item(&mut self, item_ref: &ItemRef) -> NullResult {
        let item_ = item_ref.borrow();
        let ident = item_.ident();
        trace(&format!("adding item '{}' to store: {:#?}", &ident, &item_));
        if self.known.contains_key(&ident) {
            return Err(fanling_error!(&format!("duplicate ident '{}'", &ident)));
        }
        self.known.insert(ident.clone(), item_ref.clone());
        let blob = item_.to_yaml()?;
        //  let oid = self.repo.notify_blob(&blob)?;
        self.pending_changes.push(Change::new(
            ObjectOperation::Add(String::from_utf8_lossy(&blob).to_string()),
            self.path_from_ident(&ident),
            format!("add {}", &ident),
        ));
        self.apply_changes()?;
        //     .apply_changes(&self.pending_changes, &format!("add item {}", &ident))?;
        // self.pending_changes.clear();
        //  self.get_item_parent
        Ok(())
    }
    /** ensure that all pending changes are actioned */
    pub fn apply_changes(&mut self) -> NullResult {
        self.repo.apply_changes(&self.pending_changes)?;
        self.pending_changes.clear();
        self.set_needs_push();
        Ok(())
    }
    /** mark an [`Item`] as modified */
    pub fn mark_item_modified(&mut self, item_: &mut Item) -> NullResult {
        //    let item_ = item_ref.borrow();
        let ident = item_.ident();
        fanling_trace!(&format!(
            "modifying item '{}' in store to {:?}",
            ident, &item_
        ));
        if !self.known.contains_key(&ident) {
            trace(&format!(
                "known idents are {:?}, could not find '{}'",
                self.known.keys(),
                &ident
            ));
            return Err(fanling_error!(&format!("ident '{}' not known", &ident)));
        }
        let blob = item_.to_yaml()?;
        //  let oid = self.repo.notify_blob(&blob)?;
        self.pending_changes.push(Change::new(
            ObjectOperation::Modify(String::from_utf8_lossy(&blob).to_string()),
            self.path_from_ident(&ident),
            format!("modify {}", &ident),
        ));
        self.apply_changes()?;
        Ok(())
    }
    /** mark an [`Item`] as deleted */
    pub fn mark_item_deleted(&mut self, item: ItemRef) -> NullResult {
        let item_ = item.borrow();
        let ident = item_.ident();
        fanling_trace!(&format!("deleting item '{}' in store", ident));
        // if !self.known.contains_key(&ident) {
        //     Err(fanling_error!(&format!("missing ident {}", &ident)))?
        // }
        let v = self.known.remove(&ident);
        if v.is_none() {
            return Err(fanling_error!(&format!("missing ident {}", &ident)));
        }
        self.pending_changes.push(Change::new(
            ObjectOperation::Delete,
            self.path_from_ident(&ident),
            format!("delete {}", &ident),
        ));
        self.apply_changes()?;
        Ok(())
    }
    /** get an item if it is already known. (This can be used to check whether an item is known). */
    pub fn get_item_if_known(&self, ident: &Ident) -> Option<&ItemRef> {
        // debug code
        for item in self.list_all_items().expect("bad???") {
            trace(&format!("item: {}", item.path));
        }
        trace(&format!("known: {:#?}", self.known));
        // end debug code

        self.known.get(ident)
    }
    /** check if tree contains file by path */
    pub fn has_file(&self, ident: &str) -> FLResult<bool> {
        Ok(self
            .repo
            .repo_has_file(&self.path_from_ident(&ident.to_owned()))?)
    }
    /** get the raw data for the item, both the parts common to all types of item and the parts specific to this kind of item. */
    pub fn get_item_parts(&self, ident: &Ident) -> FLResult<(ItemBaseForSerde, serde_yaml::Value)> {
        fanling_trace!(&format!("getting parts of '{}'", ident));
        assert_ne!(ident, "", "Ident is blank");
        //   let data = self.repo.blob_from_path(&self.path_from_ident(ident))?;
        let data = self.get_serialised(ident)?;
        // let serde_value: serde_yaml::Value = serde_yaml::from_slice(&data)?;
        // let base: ItemBaseForSerde = serde_yaml::from_value(serde_value.clone())?;
        // trace("got parts.");
        // Ok((base, serde_value.clone()))
        crate::item::split_data_parts(&data)
    }
    /** get the serialised data for an item */
    fn get_serialised(&self, ident: &Ident) -> FLResult<Vec<u8>> {
        Ok(self.repo.blob_from_path(&self.path_from_ident(ident))?)
    }
    /** create an [`Item`] from YAML and add it to the known map. */
    pub fn make_known(&mut self, item_rcrc: ItemRef) -> FLResult<ItemRef> {
        // let mut item = item_type.make_raw();
        // item.set_from_yaml(serde_value, world)?;
        // item.set_from_serde(base)?;
        // assert!(
        //     item.ident() == "" || item.ident() == *base.ident,
        //     "bad ident {} expected {}",
        //     item.ident(),
        //     base.ident
        // );
        // item.set_ident(base.ident.clone());
        // let item_rcrc = Rc::new(RefCell::new(item));
        let ident = item_rcrc.deref().borrow().ident();
        // let _keys: Vec<String> = self.known.keys().map(|sr| sr.deref().to_string()).collect();
        //  trace(&format!("known to store are {}", keys.join(", ")));
        //  assert!(!self.known.contains_key(ident), "duplicate key {}", &ident);
        // TODO check that any dups are the same item
        if self.known.contains_key(&ident) {
            Ok(self
                .known
                .get(&ident)
                .ok_or_else(|| fanling_error!(&format!("ident not found {}", &ident)))?
                .clone()) //
        } else {
            self.known.insert(ident, item_rcrc.clone());
            Ok(item_rcrc)
        }
    }
    /** set the next ident num */
    pub fn set_next_ident_num(&mut self, next_ident_num: i64) {
        self.next_ident_num = next_ident_num.try_into().expect("bad???");
    }
    /** get the next ident num */
    pub fn get_next_ident_num(&self) -> i64 {
        self.next_ident_num.try_into().expect("bad???")
    }

    /** make a new [`Ident`] */
    pub fn make_identifier(&mut self, uniq_pfx: &str, name: &str) -> Ident {
        trace(&format!(
            "making ident from '{}' prefix '{}'",
            name, uniq_pfx
        ));
        assert!(!uniq_pfx.is_empty(), "prefix is empty");
        assert!(!name.is_empty(), "name is empty");
        let tnx = self.unallowed_chars.replace_all(name, "-");
        let tidy_name = self.initial_dash.replace(&tnx, "");
        match tidy_name.chars().next() {
            None => panic!("name should not resolve to empty (name was '{}')", name),
            Some(ch0) => assert!(ch0 != '-' && ch0 != '?', "bad name '{}'", &tidy_name),
        }
        let max_len = 20;
        let short_tidy_name = if tidy_name.len() < max_len {
            tidy_name.into_owned()
        } else {
            tidy_name[..max_len].to_string()
        };
        self.next_ident_num += 1;
        format!("{}-{}{}", short_tidy_name, uniq_pfx, &self.next_ident_num)
    }
    /** returns a list of all items */
    pub fn list_all_items(&self) -> FLResult<Vec<EntryDescr>> {
        let items: Vec<EntryDescr> = self
            .repo
            .list_all()?
            .into_iter()
            .filter(|i| self.path_has_ident(&i.path))
            .collect();
        trace(&format!("{} entries found in repo", items.len()));
        Ok(items)
    }
    // /** pull any new changes from the server */
    // pub fn pull(&mut self) -> NullResult {
    //     unimplemented!()
    // }
    /** push the store to the server if required */
    pub fn push(&mut self, force: bool) -> NullResult {
        let needs_pushing = self.repo.does_need_pushing();
        fanling_trace!(&format!(
            "push: {} from {}",
            if needs_pushing {
                "needs push"
            } else {
                "does not need push"
            },
            self.repo.trace_descr()
        ));
        if needs_pushing {
            self.repo.push(force)?;
        }
        trace(&format!(
            "after push, needs push {:?}",
            self.get_needs_push()
        ));
        Ok(())
    }
    /** given an ident, form the path within the repo */
    pub fn path_from_ident(&self, ident: &Ident) -> String {
        format!("{}.page", ident)
    }
    /** given a path within the repo, find whether it could refer to
    an [Item] and if so retrieve the [Ident] */
    pub fn ident_from_path(&self, path: &str) -> Option<Ident> {
        trace(&format!("path is {}", path));
        match self.item_path_re.captures(path) {
            None => {
                trace("no match");
                None
            }
            Some(cc) => Some(cc.get(1).map_or("".to_string(), |m| m.as_str().to_string())),
        }
    }
    /**  given a path within the repo, find whether it could refer to
    an [Item] */
    pub fn path_has_ident(&self, path: &str) -> bool {
        self.item_path_re.captures(path).is_some()
    }
    /** fetch from server */
    pub fn fetch(&mut self) -> NullResult {
        Ok(self.repo.fetch()?)
    }
    /** merge the versions and determine the status (no change/fast forward/conflict) */
    pub fn merge(&mut self) -> FLResult<MergeOutcome> {
        Ok(self.repo.merge()?)
    }

    ///** latest local commit for fetch */
    // pub fn our_commit(&self) -> FLResult<Commit> {
    //     Ok(self.repo.our_commit()?)
    // }
    // /** latest commit on other branch after fetch */
    // fn their_commit(&self) -> FLResult<Commit> {
    //     Ok(self.repo.their_commit()?)
    // }
    /** set that the repo needs to be pushed */
    pub fn set_needs_push(&mut self) {
        self.repo.set_needs_push();
    }
    /** get whether the repo needs to be pushed */
    pub fn get_needs_push(&self) -> bool {
        self.repo.does_need_pushing()
    }
    /** Apply commit after merge */
    pub fn commit_merge(&mut self, mo: &mut MergeOutcome) -> NullResult {
        trace("committing merge...");
        self.repo.commit_merge(mo)?;
        Ok(())
    }
    /**  the conflicts, if any, resulting from the merge */
    pub fn conflicts(&self, mo: &MergeOutcome) -> FLResult<ConflictList> {
        Ok(self.repo.conflicts(mo)?)
    }
    /** apply a change list to an index (to resolve conflicts) */
    // pub fn apply_changelist_to_index(&self, changes: &ChangeList, index: &mut Index) -> NullResult {
    //    Ok( self.repo.apply_changelist_to_index(changes, index)?)
    // }
    //}
    /** apply a change list to a merge outcome (to resolve conflicts) */
    pub fn apply_changes_to_merge_outcome(
        &mut self,
        c: &ChangeList,
        mo: &mut MergeOutcome,
    ) -> NullResult {
        self.set_needs_push();
        Ok(self.repo.apply_changes_to_merge_outcome(c, mo)?)
    }
    /** whether the repository has a remote */
    pub fn has_remote(&self) -> bool {
        self.repo.has_remote()
    }
    /** a description identifying the engine for use in diagnostic
    traces */
    pub fn trace_descr(&self) -> String {
        self.repo.trace_descr()
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        trace("dropping store");
        assert_eq!(0, self.pending_changes.len());
    }
}
/** convenience function for debug traces */
fn trace(txt: &str) {
    trace!("{}", txt);
    println!(
        "store {}",
        ansi_term::Colour::Blue
            .on(ansi_term::Colour::Yellow)
            .paint(txt)
    );
}
