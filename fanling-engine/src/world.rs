/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! implement the top level of the model */
use crate::fanling_error;
use crate::fanling_trace;
use crate::item::{
    split_data_parts, Ident, Item, ItemBaseForSerde, ItemKind, ItemLink, ItemListEntryList,
    ItemRef, ItemType, SpecialKind,
};
use crate::search::Search;
use crate::shared::{FLResult, FanlingError, NullResult, Tracer};
use crate::store::Store;
use askama::Template;
use fanling_interface::error_response_result;
use log::trace;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::Deref;
use std::rc::Rc;
use taipo_git_control::{MergeOutcome, RepoActionRequired};

/** this is the model class that does the actual work */
pub struct World {
    /** the store containing the data */
    store: Store,
    /** search (database) */
    search: Search,
    /** the item types */
    item_type_registry: crate::item::ItemTypeRegistry,
    /** type of interface that is connected to the engine*/
    interface_type: crate::InterfaceType,
    /** unique prefix for items created on this instance to ensure that idents aer unique */
    uniq_pfx: String,
    /** context to use when creating a new task */
    default_context: Option<ItemRef>,
    /** automatically generate items for missing items in links */
    auto_link: bool,
}
impl<'a> World {
    /** create a new [World]  */
    pub fn new_and_open(opts: &'a crate::EngineOptions) -> Result<Self, FanlingError> {
        fanling_trace!("creating world");
        let mut item_type_registry = crate::item::ItemTypeRegistry::new();
        let simple_itr = crate::item::ItemType::new(crate::simple::SimpleTypePolicy::new_boxed());
        item_type_registry.register(simple_itr);
        let task_itr = crate::item::ItemType::new(crate::task::TaskTypePolicy::new_boxed());
        item_type_registry.register(task_itr);
        let (search, _new_db) = Search::new_and_open(&opts.search_options)?;
        let (last_ident, _ident_prefix) = search.read_global()?;
        let (mut store, repo_action_required) = Store::new_and_open(&opts.repo_options)?;
        store.set_next_ident_num(last_ident.into());
        let mut world = Self {
            store,
            search,
            item_type_registry,
            interface_type: opts.interface_type,
            uniq_pfx: opts.uniq_pfx.clone(),
            default_context: None,
            auto_link: opts.auto_link,
        };
        // if new_db {
        //     world.get_all()?;
        // }
        if opts.repo_options.url.is_some() {
            trace("fetching changes...");
            world.process_fetch_changes(repo_action_required)?;
        }
        trace("ensuring some items...");
        world.ensure_some_items()?;
        trace("created world.");
        Ok(world)
    }
    /** handle any changes to the data that come from the new state of
    the repository after a fetch */
    fn process_fetch_changes(&mut self, repo_action_required: RepoActionRequired) -> NullResult {
        fanling_trace!(&format!(
            "processing fetch changes {:?}",
            repo_action_required
        ));
        match repo_action_required {
            RepoActionRequired::NoAction => Ok(()),
            RepoActionRequired::LoadAll => {
                fanling_trace!("loading all items");
                for entry in self.store.list_all_items()? {
                    // trace(&format!("should load {:?}", entry));
                    let (base, values) = split_data_parts(entry.blob.as_bytes())?;
                    let ident = self.make_known(&values, base)?;
                    let path_from_ident = self.store.path_from_ident(&ident);
                    if path_from_ident != entry.path {
                        return Err(fanling_error!(&format!(
                            "bad path {} vs {} for {}",
                            path_from_ident, entry.path, ident
                        )));
                    }
                }
                Ok(())
            }
            RepoActionRequired::ProcessChanges => {
                // fanling_trace!("fetching");
                // if self.store.has_remote() {
                //     self.store.fetch()?;
                //     let mut merge_outcome = self.store.merge()?;
                //     //    trace(&format!("fetch result was {:?}", merge_outcome,));
                //     match merge_outcome {
                //         MergeOutcome::AlreadyUpToDate => {}
                //         MergeOutcome::Merged | MergeOutcome::Conflict(_) => {
                //             self.store.set_needs_push();
                //             self.handle_merge_outcome(&mut merge_outcome)?;
                //             self.store.commit_merge(&mut merge_outcome)?;
                //         }
                //     }
                // }
                // Ok(())
                self.pull()
            }
        }
    }
    fn pull(&mut self) -> NullResult {
        fanling_trace!("pulling");
        if self.store.has_remote() {
            fanling_trace!("fetching");
            let fr: NullResult = self.store.fetch();
            if fr.is_err() {
                trace(&format!("fetch was error ({:#?})", &fr));
            }
            fanling_trace!(&format!("fetch result {:#?}", &fr));
            fr?;
            let mut merge_outcome = self.store.merge()?;
            trace(&format!("fetch result was {:?}", merge_outcome,));
            match merge_outcome {
                MergeOutcome::AlreadyUpToDate => {}
                MergeOutcome::Merged(_) | MergeOutcome::Conflict(_) => {
                    self.store.set_needs_push();
                    self.handle_merge_outcome(&mut merge_outcome)?;
                    self.store.commit_merge(&mut merge_outcome)?;
                }
            }
        }
        Ok(())
    }

    /** handle the result of the merge */
    pub fn handle_merge_outcome(&mut self, mo: &mut MergeOutcome) -> NullResult {
        fanling_trace!(&format!("handling merge outcome {:?}", mo));
        let mut changes = vec![];
        for conflict in &self.store.conflicts(mo)? {
            trace(&format!("conflict: {:?}", &conflict));
            let mut type_name: Option<String> = None;
            let anc = match &conflict.ancestor {
                Some(ie) => {
                    let (base, _value) = split_data_parts(&ie.data.as_slice())?;
                    type_name = Some(base.type_name.clone());
                    Some(base.clone())
                }
                _ => None,
            };
            let our = match &conflict.our {
                Some(ie) => {
                    let (base, _value) = split_data_parts(&ie.data.as_slice())?;
                    type_name = Some(base.type_name.clone());
                    Some(base)
                }
                _ => None,
            };
            let their = match &conflict.their {
                Some(ie) => {
                    let (base, _value) = split_data_parts(&ie.data.as_slice())?;
                    type_name = Some(base.type_name.clone());
                    Some(base)
                }
                _ => None,
            };
            if let Some(a) = anc.clone() {
                if let Some(o) = our.clone() {
                    assert_eq!(a.type_name, o.type_name);
                }
                if let Some(t) = their.clone() {
                    assert_eq!(a.type_name, t.type_name);
                }
            }
            if let Some(o) = our.clone() {
                if let Some(t) = their.clone() {
                    assert_eq!(o.type_name, t.type_name);
                }
            }
            if let Some(itn) = type_name {
                let item_type = self.get_item_type(itn)?;
                item_type
                    .deref()
                    .borrow()
                    .resolve_conflict(self, &conflict, &mut changes)?;
            }
        }
        match mo {
            MergeOutcome::AlreadyUpToDate => {
                // BUG: if merged, need to commit
                // Err(fanling_error!(&format!("bad merge outcome {:?}", mo)))
                trace("merge outcome was up to date");
                fanling_trace!("merge outcome up to date, no action required");
                Ok(())
            }
            MergeOutcome::Merged(_) => {
                // BUG: if merged, need to commit
                // Err(fanling_error!(&format!("bad merge outcome {:?}", mo)))
                trace("merge outcome was merged");
                fanling_trace!("merge outcome merged, no action required TODO check");
                Ok(())
            }
            MergeOutcome::Conflict(_) => {
                trace("merge outcome was conflict");
                // self.store.apply_changelist_to_index(&changes, ix)?;
                self.store.apply_changes_to_merge_outcome(&changes, mo)?;
                Ok(())
            }
        }
    }
    /** ensure that some items exist */
    fn ensure_some_items(&mut self) -> NullResult {
        let contexts = self.search_contexts()?.entries;
        trace(&format!("contexts are {:?}", &contexts));
        /* FUTURE seemed to be generating default context even when one already exists. This sems to be fixed now, but check. */
        if contexts.is_empty() {
            self.ensure_some_context()?;
        }
        Ok(())
    }
    /** ensure there is a context */
    fn ensure_some_context(&mut self) -> NullResult {
        trace("no contexts, so creating one");
        // let type_name = "Simple".to_owned();
        // let base = ItemBaseForSerde {
        //     ident: "default_context".to_owned(),
        //     type_name: type_name.clone(),
        //     can_be_context: true,
        //     ..ItemBaseForSerde::default()
        // };
        // let mut vals = HashMap::new();
        // vals.insert("name".to_owned(), "Default context".to_owned());
        // self.default_context = Some(self.make_item(&type_name, &base, &vals)?);
        self.default_context =
            Some(self.ensure_item("default_context".to_owned(), "Simple".to_string())?);
        trace("created context.");
        Ok(())
    }
    /** create an item */
    fn ensure_item(&mut self, ident: Ident, type_name: Ident) -> FLResult<ItemRef> {
        //  let type_name = "Simple".to_owned();
        let base = ItemBaseForSerde {
            ident: ident.to_owned(),
            type_name: type_name.clone(),
            can_be_context: type_name == "Simple",
            can_be_parent: true,
            ..ItemBaseForSerde::default()
        };
        let mut vals = HashMap::new();
        vals.insert("name".to_owned(), ident);
        match type_name.as_str() {
            "Simple" => {}
            "Task" => {
                vals.insert("context".to_string(), "default_context".to_string());
            }
            _ => return Err(fanling_error!(&format!("invalid type '{}'", &type_name))),
        }
        let res = self.make_item(&type_name, &base, &vals)?;
        Ok(res)
    }
    /** get the default context */
    pub fn get_default_context(&self) -> ItemRef {
        self.default_context.clone().expect("bad???")
    }
    /** get the default context for a list */
    pub fn get_default_context_for_list(&self) -> FLResult<ItemLink> {
        match self.default_context.clone() {
            None => Err(fanling_error!("no default context")),
            Some(c) => Ok(ItemLink::from(c)),
        }
    }
    /** interpret an ident as an item kind */
    pub fn item_kind(type_ident: &Ident) -> ItemKind {
        match type_ident.as_str() {
            "simple" | "Simple" => ItemKind::Simple,
            "task" | "Task" | "todo" => ItemKind::Task,
            _ => panic!(format!("bad type ident: {}", &type_ident)),
        }
    }
    /** make an [`Item`] and add it to the store and search */
    pub fn make_item(
        &mut self,
        type_name: &str,
        base: &ItemBaseForSerde,
        vals: &HashMap<String, String>,
    ) -> crate::shared::FLResult<crate::item::ItemRef> {
        // let item_type_rcrc: Rc<RefCell<ItemType>> = self
        //     .item_type_registry
        //     .get(Self::item_kind(&type_name.to_string()))?;
        trace(&format!("making {} item...", type_name));
        let item_type_rcrc = self.get_item_type(type_name.to_owned())?;
        let item_type = item_type_rcrc.deref().borrow();
        trace(&format!("item type is {}, making...", item_type.ident()));
        let mut item = item_type.make_raw();
        //  let mut item: Item = ItemType::from(*item_type_rcrc.borrow()).make_raw();
        item.set_data(vals, self)?;
        let descr = item.descr_for_ident();
        if descr.is_empty() {
            return Err(fanling_error!("description must not be blank"));
        }
        item.set_ident(self.store.make_identifier(&self.uniq_pfx, &descr));
        assert!(item.ident() != "", "ident is null");
        self.search
            .update_last_ident(self.store.get_next_ident_num().try_into()?)?;
        // self.set_parent_from_ident(&mut item, base)?;
        item.set_from_serde(base)?;

        trace(&format!("item as created {:#?}", item));
        let item_rcrc = Rc::new(RefCell::new(item));
        self.store.add_item(&item_rcrc)?;
        self.search.add_item(&item_rcrc)?;
        trace("made item.");
        Ok(item_rcrc)
    }
    /** get the item type with a given name */
    pub fn get_item_type(&mut self, type_name: Ident) -> FLResult<Rc<RefCell<ItemType>>> {
        Ok(self.item_type_registry.get(Self::item_kind(&type_name))?)
        //  Ok(item_type_rf.deref().borrow())
    }
    /** check that an item would be valid */
    pub fn check_item_valid(
        &mut self,
        //   ident: Ident,
        item_type_rf: Rc<RefCell<ItemType>>,
        base: &ItemBaseForSerde,
        vals: &HashMap<String, String>,
    ) -> FLResult<ActionResponse> {
        let mut item_type = item_type_rf.deref().borrow_mut();
        //    let item = self.get_item(ident)?;
        let ar = item_type.check_valid(base, vals, self);
        Ok(ar)
    }
    // /** check that an item would be valid */
    // fn check_valid(
    //     &mut self,
    //     item: ItemRef,
    //     base: &ItemBaseForSerde,
    //     vals: &HashMap<String, String>,
    // ) -> ActionResponse {
    //     item.deref().borrow_mut().check_valid(base, vals, self)
    // }
    /** carry out an action and return a [`Response`]. May delegate the action to a component. */
    pub fn do_action(
        &mut self,
        basic_request: &crate::BasicRequest,
        _json_value: serde_json::value::Value,
    ) -> fanling_interface::ResponseResult {
        let mut res = match basic_request.action.kind() {
            crate::ActionKind::Engine => error_response_result("should not come here"),
            crate::ActionKind::World => self.do_world_action(basic_request),
            crate::ActionKind::Item => {
                let ident: Ident = basic_request
                    .ident
                    .as_ref()
                    .ok_or_else(|| fanling_error!("need ident here"))?
                    .to_string();
                let item_rf = self.get_item(ident, "Simple".to_owned())?;
                let item: &mut Item = &mut item_rf.deref().borrow_mut();
                let res = item.do_action(basic_request.action.clone(), self)?;
                trace("item action done");
                Ok(res)
            }
        }?;
        self.add_always(&mut res)?;
        trace("action done");
        Ok(res)
    }
    /** get an [`Item`] by [`Ident`] */
    pub fn get_item(&mut self, ident: Ident, type_name: Ident) -> FLResult<ItemRef> {
        trace(&format!("getting item '{}'", ident));
        match self.store.get_item_if_known(&ident) {
            Some(i) => Ok(i.clone()),
            None => {
                //  if let Some(type_name) = type_option {
                if self.auto_link
                    && !(self
                        .store
                        .has_file(&ident)
                        .expect("error when checking if page in store"))
                {
                    self.ensure_item(ident.clone(), type_name)?;
                }
                let (base, serde_value) = self.store.get_item_parts(&ident)?;
                let item_ref = self.get_and_make_known(&serde_value, &base)?;
                Ok(item_ref)
            }
        }
    }
    /** take the raw data for an item and ensure that the item is ready to use */
    fn get_and_make_known(
        &mut self,
        serde_value: &serde_yaml::Value,
        base: &ItemBaseForSerde,
    ) -> FLResult<ItemRef> {
        fanling_trace!("getting and making known");
        let item_type_rcrc = self.get_item_type(base.type_name.to_owned())?;
        let item_type = item_type_rcrc.deref().borrow();
        let item_ref = self.make_and_populate_item(&item_type, &base, serde_value)?;
        trace("got item.");
        Ok(self.store.make_known(item_ref)?)
        // item_ref.deref().borrow_mut().set_from_serde(&base, self)?;
        // Ok(item_ref)
    }
    /** make an item and set its fields  */
    pub fn make_and_populate_item(
        &mut self,
        item_type: &ItemType,
        base: &ItemBaseForSerde,
        serde_value: &serde_yaml::Value,
    ) -> FLResult<ItemRef> {
        fanling_trace!(&format!("making and populating item {}", &base.ident));
        let mut item = item_type.make_raw();
        trace("setting base...");
        item.set_from_yaml(serde_value, self)?;
        trace("setting data...");
        item.set_from_serde(base)?;
        assert!(
            item.ident() == "" || item.ident() == *base.ident,
            "bad ident {} expected {}",
            item.ident(),
            base.ident
        );
        trace(&format!("setting ident ({})...", base.ident));
        item.set_ident(base.ident.clone());
        item.fix_data(serde_value, self)?;
        trace("item made and populated");
        Ok(Rc::new(RefCell::new(item)))
    }
    /** resolves an [`ItemLink`] to point to an [`Item`] */
    pub fn resolve_link(&mut self, item_link: &mut ItemLink) -> FLResult<ItemRef> {
        item_link.resolve_link(self)
    }

    /** add content to push to id=always */
    pub fn add_always(&self, res: &mut fanling_interface::Response) -> NullResult {
        let at = AlwaysTemplate {
            needs_push: self.store.does_need_pushing(),
        };
        res.add_tag("always", &(at.render()?));
        Ok(())
    }
    /** carry out an action and return a [`Response`]. Does not delegate. */
    pub fn do_world_action(
        &mut self,
        basic_request: &crate::BasicRequest,
        //    _json_value: serde_json::value::Value,
    ) -> fanling_interface::ResponseResult {
        match &basic_request.action {
            crate::Action::Start | crate::Action::ListReady => {
                let mut open = self.search.search_open_hier()?;
                let mut ready = open.filter_on_item(|i, world| i.is_ready(world), self)?;
                Self::show_list(&mut ready, "ready")
            }
            crate::Action::Create(base, vals) => {
                let type_name = basic_request.ensure_type_name()?;
                let item_type_rf = self.get_item_type(type_name.clone())?;
                let action_result = self.check_item_valid(item_type_rf, base, vals)?;
                if let ActionResponse::Failure {
                    messages: _,
                    specifics: _,
                } = action_result
                {
                    return action_result.to_response();
                }
                let item_ref = self.make_item(&type_name, &base, &vals)?;
                let res = item_ref.deref().borrow_mut().for_edit(true, self);
                fanling_trace!("action done");
                res
            }
            crate::Action::Update(base, vals) => {
                let res = self.update_item_action(basic_request, &base, vals)?;
                fanling_trace!("action done");
                Ok(res)
            }
            crate::Action::Delete => self.delete_item_action(basic_request),
            crate::Action::GetAll => self.get_all(),
            crate::Action::CheckData => self.check_data(),
            crate::Action::ListOpen => {
                let mut open = self.search.search_open_hier()?;
                Self::show_list(&mut open, "open")
            }
            crate::Action::ListAll => {
                let mut all = self.search.search_all_hier()?;
                Self::show_list(&mut all, "all")
            }
            crate::Action::Pull => {
                trace("doing pull action");
                self.pull()?;
                fanling_trace!("action done");
                Ok(fanling_interface::Response::new())
            }
            crate::Action::Push { force } => {
                trace("doing push action");
                self.store.push(*force)?;
                trace(&format!(
                    "after push, needs push: {:?}",
                    self.store.get_needs_push()
                ));
                fanling_trace!("action done");
                Ok(fanling_interface::Response::new())
            }
            crate::Action::New => {
                let item_type_name: Ident = basic_request.ensure_type_name()?;
                trace(&format!("new for item type {}", item_type_name));
                let item_type = self.get_item_type(item_type_name)?;
                let mut item = item_type.deref().borrow().make_raw();
                fanling_trace!("action done");
                item.for_edit(false, self)
            }
            crate::Action::NewChild(parent_ident) => {
                let item_type_name: Ident = basic_request.ensure_type_name()?;
                let item_type = self.get_item_type(item_type_name)?;
                let mut item = item_type.deref().borrow().make_raw();
                let base = ItemBaseForSerde {
                    parent: Some(parent_ident.to_string()),
                    ..ItemBaseForSerde::default()
                };
                // self.set_parent_from_ident(&mut item, &base)?;
                item.set_from_serde(&base)?;
                fanling_trace!("action done");
                item.for_edit(false, self)
            }
            crate::Action::Clone => {
                let res = self.clone_item(basic_request);
                fanling_trace!("action done");
                res
            }
            crate::Action::TestError2 => {
                trace("making world test error 2");
                Err(Box::new(fanling_error!("test error 2")))
            }
            _ => error_response_result(&format!("invalid action {:?}", basic_request.action)),
        }
    }
    /** show a list of items */
    fn show_list(list: &mut ItemListEntryList, narr: &str) -> fanling_interface::ResponseResult {
        list.set_level_changes();
        trace(&format!(
            "{}: {} entries {:?}",
            narr,
            list.entries.len(),
            &list
        ));
        #[cfg(test)]
        let entries_count = list.num_entries();
        let lt = ListTemplate {
            items: list.clone(),
        };
        let mut resp = fanling_interface::Response::new();
        resp.add_tag("content", &(lt.render()?));
        #[cfg(test)]
        resp.set_test_data("count", &format!("{}", entries_count));
        //   trace(&format!("list list {:?}", &resp));
        fanling_trace!("showing list");
        Ok(resp)
    }
    /** update an item */
    fn update_item_action(
        &mut self,
        basic_request: &crate::BasicRequest,
        base: &ItemBaseForSerde,
        vals: &HashMap<String, String>,
    ) -> fanling_interface::ResponseResult {
        let ident: Ident = basic_request.ensure_ident()?;
        let type_name = basic_request.ensure_type_name()?;
        let item_type_rf = self.get_item_type(type_name)?;
        let action_result = self.check_item_valid(item_type_rf, base, vals)?;
        trace(&format!("action result is {:#?}", action_result));
        if action_result.ok() {
            let item_rf = self.get_item(ident, "Simple".to_owned())?;
            let mut item = item_rf.deref().borrow_mut();
            trace(&format!("values for base update: {:#?}", base));
            item.set_from_serde(base)?;
            trace(&format!("values for data update: {:#?}", vals));
            item.set_data(vals, self)?;
            // self.search.check_item_valid(&mut item)?;
            // self.store.mark_item_modified(&mut item)?;
            trace("persisting change for ok update action");
            self.persist_change(&mut item)?;
        }
        Ok(action_result.to_response()?)
    }
    /** clone an item */
    fn clone_item(
        &mut self,
        basic_request: &crate::BasicRequest,
    ) -> fanling_interface::ResponseResult {
        let existing_ident: Ident = basic_request.ensure_ident()?;
        let existing_item_rf = self.get_item(existing_ident, "Simple".to_owned())?;
        let existing_item = existing_item_rf.deref().borrow_mut();
        let item_type_rf = existing_item.item_type();
        let item_type = item_type_rf.deref().borrow();
        let mut item = item_type.make_raw();
        item.clone_from(&existing_item)?;
        item.set_ident(
            self.store
                .make_identifier(&self.uniq_pfx, &item.descr_for_ident()),
        );
        assert!(item.ident() != "", "ident is null");
        self.search
            .update_last_ident(self.store.get_next_ident_num().try_into()?)?;
        let item_rcrc = Rc::new(RefCell::new(item));
        self.store.add_item(&item_rcrc)?;
        self.search.add_item(&item_rcrc)?;
        let mut item_ref = item_rcrc.deref().borrow_mut();
        Ok(item_ref.for_edit(true, self)?)
    }
    /** write out any changes to the search database and the store */
    pub fn persist_change(&mut self, item: &mut Item) -> NullResult {
        trace(&format!("persisting change for '{}'", item.ident()));
        self.search.update_item(item)?;
        self.store.mark_item_modified(item)?;
        Ok(())
    }
    /** delete an item and ensure that the store and the search are updated accordingly */
    fn delete_item_action(
        &mut self,
        basic_request: &crate::BasicRequest,
    ) -> fanling_interface::ResponseResult {
        let ident: Ident = basic_request.ensure_ident()?;
        let type_name = basic_request.ensure_type_name()?;
        let _item_type_rf = self.get_item_type(type_name)?;
        // TODO check whether item can be deleted
        let item_rf = self.get_item(ident, "Simple".to_owned())?;
        self.search.delete_item(item_rf.clone())?;
        self.store.mark_item_deleted(item_rf)?;
        Ok(fanling_interface::Response::new())
    }
    /** process all items in the store */
    fn get_all(&mut self) -> fanling_interface::ResponseResult {
        fanling_trace!("getting items into store...");
        self.search.clear()?;
        self.store.clear_known();
        for ed in self.store.list_all_items()?.iter() {
            let ident_opt = self.store.ident_from_path(&ed.path);
            match ident_opt {
                None => {
                    trace(&format!("no ident for {}, skipping: {:?}", ed.path, &ed));
                }
                Some(ident_from_path) => {
                    fanling_trace!(&format!(
                        "{} item {:?}->{:?}: {:?}",
                        ed.kind, ed.path, ident_from_path, &ed
                    ));
                    let yaml = if ed.blob.len() < 4 || &ed.blob[0..4] != "---\n" {
                        format!("---\n{}", &ed.blob)
                    } else {
                        ed.blob.to_string()
                    };
                    //  let trimmed_yaml = yaml.trim_end_matches("\n");
                    let serde_value_result = serde_yaml::from_str(&yaml);
                    if let Err(e) = &serde_value_result {
                        fanling_trace!(&format!(
                            "yaml deserialize error: {:?} at {}:{}",
                            e,
                            file!(),
                            line!(),
                        ));
                    }
                    let serde_value: serde_yaml::Value = serde_value_result?;
                    trace(&format!("yaml value {:#?}", serde_value));
                    let base_result: Result<ItemBaseForSerde, serde_yaml::Error> =
                        serde_yaml::from_value(serde_value.clone());
                    match base_result {
                        Err(e) => trace(&format!("bad yaml ({:?}): \"{}\"", &e, &yaml)),
                        Ok(base) => {
                            trace(&format!("adding to search {:?}", base));
                            // let item_ref = self.get_and_make_known(serde_value, &base)?;
                            // self.search.add_item(&item_ref)?;
                            // assert_eq!(ident_from_path, item_ref.deref().borrow().ident());
                            let ident = self.make_known(&serde_value, base)?;
                            assert_eq!(ident_from_path, ident);
                        }
                    }
                }
            }
        }
        trace("got items into store");
        Ok(fanling_interface::Response::new())
    }
    /** add the item to the search engine */
    fn make_known(
        &mut self,
        serde_value: &serde_yaml::Value,
        base: ItemBaseForSerde,
    ) -> FLResult<Ident> {
        let item_ref = self.get_and_make_known(serde_value, &base)?;
        self.search.add_item(&item_ref)?;
        let ident = item_ref.deref().borrow().ident();
        Ok(ident)
    }
    /** search everything  */
    pub fn search_all(&self) -> FLResult<ItemListEntryList> {
        Ok(self.search.search_all()?)
    }
    /** search parents  */
    pub fn search_parents(&self) -> FLResult<ItemListEntryList> {
        Ok(self.search.search_special(SpecialKind::Parent)?)
    }
    /** search contexts  */
    pub fn search_contexts(&self) -> FLResult<ItemListEntryList> {
        Ok(self.search.search_special(SpecialKind::Context)?)
    }
    /** search everything for open with hierarchy */
    pub fn search_open_hier(&self) -> FLResult<ItemListEntryList> {
        self.search.search_open_hier()
    }
    /** cross-check search and store */
    fn check_data(&self) -> fanling_interface::ResponseResult {
        let all_from_search = self.search.search_all()?;
        let search_idents: Vec<String> = all_from_search
            .entries
            .iter()
            .map(|ile| ile.link.ident.clone())
            .collect();
        trace(&format!("search has {}", search_idents.join(", ")));
        let all_from_store = self.store.list_all_items()?;
        let store_paths: Vec<String> = all_from_store.iter().map(|ed| ed.path.clone()).collect();
        trace(&format!("store has {}", store_paths.join(", ")));
        let mut searched: HashMap<String, bool> = HashMap::new();
        let mut search_idents_count = 0;
        for ident in search_idents.iter() {
            searched.insert(ident.to_string(), false);
            search_idents_count += 1;
        }
        let mut missing_from_search_count = 0;
        let mut store_idents_count = 0;
        for path in store_paths.iter() {
            //    let ident_opt = self.store.ident_from_path(path);
            match self.store.ident_from_path(path) {
                None => trace(&format!("bad path {}", path)),
                Some(ident) => {
                    if !searched.contains_key(&ident) {
                        let msg = format!("{} in store repo but not search database", &ident);
                        trace(&msg);
                        missing_from_search_count += 1;
                        // #[cfg(test)]
                        // {
                        //     panic!(msg);
                        // }
                    }
                    searched.insert(ident.clone(), true);
                }
            }
            store_idents_count += 1;
        }
        let mut missing_from_store_count = 0;
        for (ident, found) in searched.iter() {
            if !found {
                let msg = format!("{} not in store repo but in search database", &ident,);
                trace(&msg);
                missing_from_store_count += 1;
                // #[cfg(test)]
                // {
                //     panic!(msg);
                // }
            }
        }
        trace(&format!(
            "{} missing from search",
            missing_from_search_count
        ));
        trace(&format!("{} missing from store", missing_from_store_count));
        trace(&format!("{} in search", search_idents_count));
        trace(&format!("{} in store", store_idents_count));
        Ok(fanling_interface::Response::new())
    }

    /** generate the initial HTML */
    pub fn initial_html(&self) -> crate::shared::FLResult<String> {
        let mt = MainTemplate {
            interface_type: self.interface_type,
            interface_type_string: format!("{:?}", self.interface_type),
        };
        Ok(mt.render()?)
    }
    /** push the store to the server */
    pub fn push(&mut self, force: bool) -> NullResult {
        self.store.push(force)
    }
    /** find all the children of this item that have open status */
    pub fn search_open_children(&self, ident: &str) -> FLResult<ItemListEntryList> {
        self.search.search_open_children(ident)
    }
    /** get an item if it is already known. (This can be used to check whether an item is known). */
    pub fn get_item_if_known(&self, ident: &Ident) -> Option<&ItemRef> {
        self.store.get_item_if_known(ident)
    }
    /** a description identifying the engine for use in diagnostic
    traces */
    pub fn trace_descr(&self) -> String {
        self.store.trace_descr()
    }
    /** separate out an item (specified by ident) into (type-independent) base and (type-dependent) values */
    pub fn get_item_parts(
        &self,
        ident: &String,
    ) -> FLResult<(ItemBaseForSerde, serde_yaml::Value)> {
        let (base, values) = self.store.get_item_parts(ident)?;
        Ok((base, values))
    }
    /** generate an error for testing the user interface */
    pub fn test_error(&self) -> FLResult<String> {
        Err(fanling_error!("test error"))
    }
}
/** template data for a list of items */
#[derive(Template)]
#[template(path = "list.html")]
struct ListTemplate {
    items: ItemListEntryList,
}
/** template data that should always be refreshed */
#[derive(Template)]
#[template(path = "always.html")]
struct AlwaysTemplate {
    needs_push: bool,
}
/** ActionResponse is the result of an update (or new item) request. */
#[derive(Eq, PartialEq, Debug)]
pub enum ActionResponse {
    /** whether the update succeeded */
    Failure {
        /**  messages to display if there are any problems */
        messages: Vec<String>,
        /** specific messages */
        specifics: Vec<(String, String)>,
    },
    Success {
        #[cfg(test)]
        /** identifier if created by the action*/
        test_data: HashMap<String, String>,
    },
}
impl ActionResponse {
    /** create a new `ActionResponse` */
    pub fn new() -> Self {
        Self::Success {
            #[cfg(test)]
            test_data: HashMap::new(),
        }
    }
    /** record that a user error has been found */
    pub fn add_error(&mut self, area: &str, m: &str) {
        match self {
            Self::Success {
                #[cfg(test)]
                    test_data: _,
            } => {
                let mut ss = Vec::new();
                ss.push((area.to_owned(), m.to_owned()));
                *self = Self::Failure {
                    messages: vec![m.to_owned()],
                    specifics: ss,
                };
            }
            Self::Failure {
                messages,
                specifics,
            } => {
                messages.push(m.to_owned());
                specifics.push((area.to_owned(), m.to_owned()));
            }
        }
    }
    /** check the condition, otherwise report an error */
    pub fn assert(&mut self, cond: bool, area: &str, m: &str) {
        if !cond {
            self.add_error(area, m);
        }
    }
    /** whether there has been no errors */
    pub fn ok(&self) -> bool {
        *self
            == Self::Success {
                #[cfg(test)]
                test_data: HashMap::new(),
            }
    }
    /** user errors */
    pub fn errors(&self) -> Vec<(String, String)> {
        match self {
            Self::Success {
                #[cfg(test)]
                    test_data: _,
            } => vec![],
            Self::Failure {
                messages: _,
                specifics,
            } => specifics.clone(),
        }
    }
    /** overall user error message */
    pub fn overall_message(&self) -> String {
        match self {
            Self::Success {
                #[cfg(test)]
                    test_data: _,
            } => "".to_owned(),
            Self::Failure {
                messages,
                specifics: _,
            } => messages.join(" "),
        }
    }
    fn to_response(&self) -> fanling_interface::ResponseResult {
        let mut response =
            fanling_interface::Response::new_with_tags(&[("message", &self.overall_message())]);
        for (t, v) in self.errors() {
            response.add_tag(&t, &v);
        }
        // if let Some(td) = self. {
        #[cfg(test)]
        response.set_all_test_data(self.get_test_data());
        //   }
        Ok(response)
    }
    #[cfg(test)]
    /** associate test data */
    pub fn set_test_data(&mut self, test_data: HashMap<String, String>) {
        match self {
            Self::Success { test_data: td } => {
                *td = test_data;
            }
            Self::Failure {
                messages: _,
                specifics: _,
            } => {}
        }
    }
    #[cfg(test)]
    /** retrieve the  [`Ident`] */
    pub fn ident(&self) -> Option<String> {
        match self {
            Self::Success { test_data: td } => Some(td.get("ident").unwrap().clone()),
            Self::Failure {
                messages: _,
                specifics: _,
            } => None,
        }
    }
    #[cfg(test)]
    /** retrieve the test data */
    pub fn get_test_data(&self) -> HashMap<String, String> {
        match self {
            Self::Success { test_data: td } => td.clone(),
            Self::Failure {
                messages: _,
                specifics: _,
            } => HashMap::new(),
        }
    }
}

/* template for initial HTML */
#[derive(Template)]
#[template(path = "main.html")]
struct MainTemplate {
    interface_type: crate::InterfaceType,
    interface_type_string: String,
}
impl Drop for World {
    fn drop(&mut self) {
        trace("dropping world");
    }
}
/** convenience function for debug traces */
fn trace(txt: &str) {
    trace!("{}", txt);
    println!(
        "world {}",
        ansi_term::Colour::White
            .on(ansi_term::Colour::Blue)
            .paint(txt)
    );
}
