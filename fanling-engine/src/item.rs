/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! implements `Items`. */

use crate::shared::{FLResult, FanlingError, NullResult, Tracer};
use crate::world::{ActionResponse, World};
use crate::Action;
use crate::{dump_fanling_error, fanling_error, fanling_trace};
use ansi_term::Colour;
use bitfield::{bitfield_bitrange, Bit};
use chrono::{NaiveDateTime, Utc};
use serde::{de::Error, Deserializer};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::fmt::Debug;
use std::ops::{Deref, Not};
use std::rc::{Rc, Weak};
use taipo_git_control::ChangeList;
use taipo_git_control::Conflict;

// bitfield! {
//     /** `Special` is a single-byte bitfield made up of bits */
//     pub struct Special(u8);
//     impl Debug;
//     u8;
//     /** one bit, read by `parent`, set by `set_parent` */
//     pub bool, parent, set_parent: 0;
//     /** one bit, read by `context`, set by `set_context` */
//     pub bool, context, set_context: 1;
// }
/** different ways in which an `Item` can be 'special' */
pub enum SpecialKind {
    /// it can be the parent of another item
    Parent,
    /// it can be the 'context' of an item
    Context,
}
impl SpecialKind {
    pub fn as_bitmap(self) -> SpecialKinds {
        let mut skk = SpecialKinds::default();
        skk.set_bit(self as usize, true);
        skk
    }
}
/** a bit field indexed by `[SpecialKind]` (a set of special kinds) */
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct SpecialKinds(u8);
bitfield_bitrange! {struct SpecialKinds(u8)}
impl Default for SpecialKinds {
    fn default() -> Self {
        SpecialKinds(0)
    }
}
impl SpecialKinds {
    pub fn val(self) -> u8 {
        self.0
    }
    pub fn has(self, sk: SpecialKind) -> bool {
        self.bit(sk as usize)
    }
    pub fn new(skk: u8) -> Self {
        Self(skk)
    }
    pub fn default_val() -> u8 {
        0
    }
    pub fn add(&mut self, sk: SpecialKind) {
        self.set_bit(sk as usize, true);
    }
}

/** data is organised into [`Item`]s */
#[derive(Debug)]
pub struct Item {
    base: ItemBase,
    /** type-specific data for the item */
    data: Box<dyn ItemData>,
}
impl Item {
    /** create a new Item with the specified item type and ItemData */
    pub fn new_with_data(item_type: ItemTypeRef, data: Box<dyn ItemData>) -> Self {
        let _naive_date_time = Utc::now().naive_utc();
        Self {
            base: ItemBase::new(item_type),
            data,
        }
    }
    /** is the  [`Item`] open? */
    pub fn is_open(&self) -> bool {
        self.data.is_open()
    }
    /** is the  [`Item`] ready to be used? */
    pub fn is_ready(&self) -> bool {
        self.data.is_ready()
    }
    /** classify the item */
    pub fn classify(&self) -> String {
        self.base.get_classify()
    }
    /** SpecialKinds */
    pub fn specials(&self) -> SpecialKinds {
        self.base.get_specials()
    }
    /** is the Item targeted */
    pub fn targeted(&self) -> bool {
        self.base.get_targeted()
    }

    /** can the  [`Item`] be used as a parent? */
    pub fn can_be_parent(&self) -> bool {
        self.base.can_be_parent()
    }

    /** do an item action (from the user interface) */
    pub fn do_action(
        &mut self,
        action: crate::Action,
        //    _json_value: serde_json::value::Value,
        world: &mut World,
    ) -> fanling_interface::ResponseResult {
        match &action {
            Action::Show => self.for_show(world),
            Action::Edit => self.for_edit(true, world),
            _ => {
                let res = self.data.do_action(&mut self.base, action, world);
                trace("persisting change for edit action");
                world.persist_change(self)?;
                res
            }
        }
    }
    // /** get the ident of the item */
    pub fn ident(&self) -> Ident {
        self.base.ident.clone()
    }
    /** set the identifier of the item */
    pub fn set_ident(&mut self, ident: Ident) {
        self.base.ident = ident;
    }
    /** get the type name of the item */
    pub fn type_name(&self) -> String {
        self.base.item_type.deref().borrow().ident()
    }
    /** get the item type of the [`Item`] */
    pub fn item_type(&self) -> ItemTypeRef {
        self.base.item_type.clone()
    }
    /** set the Item from YAML data */
    pub fn set_from_yaml(&mut self, yaml: serde_yaml::Value, world: &mut World) -> NullResult {
        fanling_trace!("setting from yaml");
        self.data.set_from_yaml(yaml, world)
    }
    /** display for editing */
    pub fn for_edit(
        &mut self,
        is_for_update: bool,
        world: &mut World,
    ) -> fanling_interface::ResponseResult {
        self.data.for_edit(&mut self.base, is_for_update, world)
    }
    /** display for show */
    pub fn for_show(&mut self, world: &mut World) -> fanling_interface::ResponseResult {
        self.data.for_show(&mut self.base, world)
    }
    /** serialise the Item to YAML */
    pub fn to_yaml(&self) -> Result<Vec<u8>, FanlingError> {
        self.data.to_yaml(&self.base)
    }
    // an English-language description
    /** a human-readable summary of the Item */
    pub fn description(&self) -> String {
        self.data.description()
    }
    /** a description that can be used in a list */
    pub fn description_for_list(&self) -> String {
        self.data.description_for_list()
    }
    /** set the ItemData from a HashMap of values */
    pub fn set_data(&mut self, vals: &HashMap<String, String>, world: &mut World) -> NullResult {
        self.data.set_data(vals, world)
    }
    // /**  check that the item would be valid */
    // pub fn check_valid(
    //     &mut self,
    //     base: &ItemBaseForSerde,
    //     vals: &HashMap<String, String>,
    //     world: &mut World,
    // ) -> ActionResponse {
    //     self.data.check_valid(base, vals, world)
    // }
    /** link to parent (if any) */
    pub fn parent(&mut self) -> Option<ItemLink> {
        self.base.parent.clone()
    }
    /** retrieve the parent ident if any */
    pub fn parent_ident(&mut self) -> Option<String> {
        match &self.base.parent {
            None => None,
            Some(il) => Some(il.ident().unwrap_or("??".to_owned())),
        }
    }
    /** set parent */
    pub fn set_parent(&mut self, parent: Option<ItemLink>) {
        self.base.set_parent(parent);
    }
    /** set base fields: parent from ident ("" means no parent) */
    pub fn set_from_serde(&mut self, base: &ItemBaseForSerde) -> NullResult {
        self.base.set_from_serde(base)
    }
    /** get the sort key for an item */
    pub fn get_sort(&self) -> String {
        self.base.sort.clone()
    }
    /** clone an item */
    pub fn clone_from(&mut self, other: &Item) -> NullResult {
        self.base.clone_from(&other.base);
        self.data = other.data.fanling_clone()?;
        Ok(())
    }
    /** assemble an item from a base and data */
    pub fn from_parts(
        oib: &ItemBaseForSerde,
        item_type: ItemTypeRef,
        os: Box<dyn ItemData>,
    ) -> FLResult<Self> {
        let mut ib = ItemBase::new(item_type);
        ib.set_from_serde(oib)?;
        Ok(Item { base: ib, data: os })
    }
    /** make an item from parts and add it to a change list as modify */
    pub fn change_using_parts(
        type_name: &str,
        ib: &ItemBaseForSerde,
        os_box: Box<dyn ItemData>,
        path: &str,
        changes: &mut ChangeList,
        world: &mut World,
    ) -> NullResult {
        let item_type_rcrc = world.get_item_type(type_name.to_string())?;
        //  let os_box: Box<ItemData> = Box::new(os);
        let item = Self::from_parts(ib, item_type_rcrc, os_box)?;
        let merged_yaml = item.to_yaml()?;
        let change = taipo_git_control::Change::new(
            taipo_git_control::ObjectOperation::Modify(
                String::from_utf8_lossy(&merged_yaml).to_string(),
            ),
            path.to_string(),
            "resolve conflict".to_owned(),
        );
        changes.push(change);
        Ok(())
    }
}
/** attributes of an [`Item`] that are the same for all `Item`s. */
#[derive(Debug, Clone)]
pub struct ItemBase {
    /** the [Ident] for this */
    ident: Ident,
    /** the item type */
    item_type: ItemTypeRef,
    /** parent link */
    parent: Option<ItemLink>,
    /** sort key */
    sort: String,
    /** a classification for the item or "normal" */
    classify: String,
    /** if the Item is special, the kind of special, otherwise 0 */
    special: SpecialKinds,
    /** whether the item is targeted for searching -- do not save this to the repo */
    targeted: bool,
    /** when the Item was created */
    when_created: NaiveDateTime,
    /** when the Item was last modified */
    when_modified: NaiveDateTime,
}
impl ItemBase {
    fn new(item_type: ItemTypeRef) -> Self {
        let naive_date_time = Utc::now().naive_utc();
        Self {
            ident: "??".to_owned(),
            item_type,
            parent: None,
            sort: "".to_owned(),
            classify: "normal".to_owned(),
            special: SpecialKinds(0),
            targeted: false,
            when_created: naive_date_time,
            when_modified: naive_date_time,
        }
    }
    pub fn get_ident(&self) -> Ident {
        self.ident.clone()
    }
    pub fn resolve_parent(&mut self, world: &mut World) -> FLResult<Option<ItemRef>> {
        Ok(match &mut self.parent {
            Some(p) => Some(world.resolve_link(p)?),
            None => None,
        })
    }
    /** get the classification */
    pub fn get_classify(&self) -> String {
        self.classify.clone()
    }
    /** get any 'specials' */
    pub fn get_specials(&self) -> SpecialKinds {
        self.special.clone() as SpecialKinds
    }
    /** whether the item can be a parent */
    pub fn can_be_parent(&self) -> bool {
        self.special.bit(SpecialKind::Parent as usize)
    }
    /**  whether the item can be a context */
    pub fn can_be_context(&self) -> bool {
        self.special.bit(SpecialKind::Context as usize)
    }
    /** whether the item is targeted */
    pub fn get_targeted(&self) -> bool {
        self.targeted
    }
    /** set parent from ident ("" means no parent) */
    pub fn set_from_serde(&mut self, base: &ItemBaseForSerde) -> NullResult {
        trace("setting base from serde...");
        let parent = match &base.parent {
            None => None,
            Some(id) => {
                if id == "" {
                    None
                } else {
                    Some(ItemLink::from(id.clone()))
                }
            }
        };
        if !base.ident.is_empty() {
            self.ident = base.ident.clone();
        }
        match self.ident.chars().next() {
            None => return Err(fanling_error!("ident should not be empty")),
            Some(ch0) => assert!(ch0 != '-' && ch0 != '?', "bad ident '{}'", &self.ident),
        }
        self.set_parent(parent);
        //  item_type is already set in make_raw()
        self.sort = base.sort.clone();
        self.classify = base.classify.clone();
        //        self.special = SpecialKinds::new(base.special);
        self.special = SpecialKinds::new(0);
        if base.can_be_parent {
            self.special.add(SpecialKind::Parent);
        }
        if base.can_be_context {
            self.special.add(SpecialKind::Context);
        }
        // do not copy targeted
        // TODO: should we copy when_created and when_modified?
        trace("set base from serde.");
        Ok(())
    }
    /** set parent */
    pub fn set_parent(&mut self, parent: Option<ItemLink>) {
        self.parent = parent;
    }
    /** can the  [`Item`] be used as a parent? */
    pub fn get_special(&self, sk: SpecialKind) -> bool {
        self.special.bit(sk as usize)
    }
    /**  data that can be used to display parents for selection in a template*/
    pub fn get_parents(&mut self, world: &mut World) -> FLResult<ItemListEntryList> {
        let mut parents = world.search_parents()?;
        parents.prepend(ItemListEntry::make_special("no parent"));
        let match_ident = match self.resolve_parent(world)? {
            Some(item) => item.deref().borrow().ident(),
            _ => "".to_owned(), /* match the no-parent option */
        };
        parents.select(match_ident);
        Ok(parents)
    }
    /** data that can be used to display the parent in a template */
    pub fn parent_for_display(&mut self, world: &mut World) -> FLResult<ItemListEntry> {
        let p = self.resolve_parent(world)?;
        let parent = match p {
            Some(p) => {
                let item: &Item = &p.deref().borrow();
                ItemListEntry::from_item(item)?
            }
            None => ItemListEntry::make_special("no parent"),
        };
        Ok(parent)
    }
    pub fn get_sort(&self) -> String {
        self.sort.clone()
    }
    /** get all the children of this [`Item`] */
    pub fn get_item_children(&self, world: &World) -> FLResult<ItemListEntryList> {
        world.search_ready_children(&self.ident)
    }
    /** copy from another base */
    pub fn clone_from(&mut self, other: &Self) {
        self.item_type = other.item_type.clone();
        self.parent = other.parent.clone();
        self.sort = other.sort.clone();
        self.classify = other.classify.clone();
        self.special = other.special.clone();
        self.targeted = other.targeted;
        self.when_created = other.when_created;
        self.when_modified = other.when_modified;
    }
}

/** interpret the serialised data as YAML and set the [ItemBase]  */
pub fn split_data_parts(data: &[u8]) -> FLResult<(ItemBaseForSerde, serde_yaml::Value)> {
    let serde_value: serde_yaml::Value = dump_fanling_error!(serde_yaml::from_slice(data));
    let base: ItemBaseForSerde = dump_fanling_error!(serde_yaml::from_value(serde_value.clone()));
    Ok((base, serde_value))
}
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
/** All ItemBase fields should be represented here. */
pub struct ItemBaseForSerde {
    /** represent the ItemBase field */
    pub ident: Ident,
    #[serde(rename = "type")]
    /** represent the ItemBase field by the ident (type name) */
    pub type_name: String,
    /**  represent the ItemBase parent field using a string */
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub parent: Option<String>,
    /**  represent the ItemBase can_be_parent field */
    #[serde(skip_serializing_if = "Not::not")]
    #[serde(default)]
    pub can_be_parent: bool,
    /**  represent the ItemBase can_be_context field */
    #[serde(skip_serializing_if = "Not::not")]
    #[serde(default)]
    pub can_be_context: bool,
    /** sort key */
    #[serde(skip_serializing_if = "std::string::String::is_empty")]
    #[serde(default)]
    pub sort: String,
    /** the classification as a string */
    #[serde(default = "ItemBaseForSerde::default_classify")]
    #[serde(skip_serializing_if = "ItemBaseForSerde::is_normal")]
    pub classify: String,
    /** should not be serialised to the repo */
    #[serde(skip)]
    #[serde(default = "ItemBaseForSerde::say_false")]
    pub targeted: bool,
    /** when the item was created */
    #[serde(default = "ItemBaseForSerde::now")]
    #[serde(alias = "whencreated")]
    #[serde(deserialize_with = "ItemBaseForSerde::deserialize")]
    pub when_created: NaiveDateTime,
    /** when the item was most recently modified */
    #[serde(default = "ItemBaseForSerde::now")]
    #[serde(deserialize_with = "ItemBaseForSerde::deserialize")]
    pub when_modified: NaiveDateTime,
    /** do not use */
    #[serde(skip)]
    pub closed: bool,
}
impl ItemBaseForSerde {
    /** default classification */
    pub fn default_classify() -> String {
        "normal".to_owned()
    }
    /** whether is default special value */
    pub fn is_default_special_val(skk: &u8) -> bool {
        *skk == 0
    }
    /** whether is normal */
    pub fn is_normal(s: &str) -> bool {
        s == "normal"
    }
    /** date/time now */
    pub fn now() -> NaiveDateTime {
        Utc::now().naive_utc()
    }
    /** false value */
    pub fn say_false() -> bool {
        false
    }
    /** gets from base */
    pub fn from_base(ib: &ItemBase) -> FLResult<Self> {
        let parent = ib.parent.clone().map(|ibp| ibp.ident().expect("bad???"));
        let naive_date_time = Utc::now().naive_utc();
        trace(&format!("parent is {:?}", &parent));
        Ok(Self {
            ident: ib.ident.clone(),
            type_name: ib.item_type.deref().borrow().ident(),
            parent,
            can_be_parent: (ib.special).clone().has(SpecialKind::Parent),
            can_be_context: (ib.special).clone().has(SpecialKind::Context),
            sort: ib.sort.clone(),
            classify: ib.classify.clone(),
            //    special: ib.special.clone().val(),
            targeted: ib.targeted,
            when_created: ib.when_created,
            when_modified: naive_date_time,
            closed: false,
        })
    }
    /** deserialise date/time from various formats (tries different formats until it finds one that works) */
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<NaiveDateTime, D::Error> {
        let time: String = Deserialize::deserialize(deserializer)?;
        let r1 =
            NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S").map_err(D::Error::custom);
        if let Ok(r) = r1 {
            //       trace("date format 1");
            return Ok(r);
        }
        let r2 =
            NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%S%.fZ").map_err(D::Error::custom);
        if let Ok(r) = r2 {
            //       trace("date format 2");
            return Ok(r);
        }
        let r3 = NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%S%.f%:z")
            .map_err(D::Error::custom);
        if let Ok(r) = r3 {
            //      trace("date format 3");
            return Ok(r);
        }
        let r4 =
            NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%S%.f").map_err(D::Error::custom);
        if let Ok(r) = r4 {
            //      trace("date format 4");
            return Ok(r);
        }
        let r99 =
            NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%S%Z").map_err(D::Error::custom);
        if r99.is_err() {
            trace(&format!("all date formats bad for '{}'", time));
        }
        r99
    }
    // /** item can not be parent, for serialisation */
    // fn is_falset(val: bool) -> bool {
    //     !val
    // }
}
impl Default for ItemBaseForSerde {
    fn default() -> Self {
        let naive_date_time = Utc::now().naive_utc();
        Self {
            ident: "??".to_owned(),
            type_name: "??".to_owned(),
            parent: None,
            can_be_parent: false,
            can_be_context: false,
            sort: "".to_owned(),
            classify: "normal".to_owned(),
            // special: 0,
            targeted: false,
            when_created: naive_date_time,
            when_modified: naive_date_time,
            closed: false,
        }
    }
}
/** type-specific data for an item */
pub trait ItemData: Debug {
    fn for_edit(
        &mut self,
        base: &mut ItemBase,
        is_for_update: bool,
        //  parent: Option<ItemRef>,
        world: &mut World,
    ) -> fanling_interface::ResponseResult;
    /** collect data that can be used in a template to show the Item */
    fn for_show(
        &mut self,
        base: &mut ItemBase,
        world: &mut World,
    ) -> fanling_interface::ResponseResult;
    /** convert the Item to YAML */
    fn to_yaml(&self, base: &ItemBase) -> Result<Vec<u8>, FanlingError>;
    /** is the  [`Item`] open? */
    fn is_open(&self) -> bool;
    /**  is the  [`Item`] ready? */
    fn is_ready(&self) -> bool;
    /** an English-language description */
    fn description(&self) -> String;
    /** copy from another item data */
    fn fanling_clone(&self) -> FLResult<Box<dyn ItemData>>;
    // where
    //     Self: Sized;
    /** a description that can be used in a list */
    fn description_for_list(&self) -> String;
    /** set the data from a hashmap of values. This can assume that all data is ok, or just return error*/
    fn set_data(&mut self, vals: &HashMap<String, String>, world: &mut World) -> NullResult;
    /** set the data from YAML data */
    fn set_from_yaml(&mut self, yaml: serde_yaml::Value, world: &mut World) -> NullResult;
    /** do an action */
    fn do_action(
        &mut self,
        base: &mut ItemBase,
        action: crate::Action,
        //    _json_value: serde_json::value::Value,
        world: &mut World,
    ) -> fanling_interface::ResponseResult;
}
/** each  [`Item`] has an `ItemType`.

Ensure that `ItemType` cannot be cloned. Ensure that the `mark` is
unique for checking purposes.  */

#[derive(Debug)]
pub struct ItemType {
    policy: Box<dyn ItemTypePolicy>,
    self_ref: Weak<RefCell<ItemType>>,
}
impl Clone for ItemType {
    /** clone the ItemType -- does not make sense as ItemTypes should be singletons (clone a reference counted pointer if necessary) */
    fn clone(&self) -> Self {
        panic!("ItemType cannot be cloned")
    }
}
impl ItemType {
    /** create a new [ItemType]  */
    pub fn new<'a>(policy: Box<dyn ItemTypePolicy>) -> ItemTypeRef {
        let new_it = Self {
            policy,
            self_ref: Weak::new(),
        };
        let itr = Rc::new(RefCell::new(new_it));
        itr.borrow_mut().self_ref = Rc::downgrade(&itr); // only here
        itr
    }
    /** get an identifier for the  ['ItemType'] */
    pub fn ident(&self) -> Ident {
        self.kind().to_string()
    }
    /** the kind of item */
    pub fn kind(&self) -> ItemKind {
        self.policy.kind()
    }
    /** make a 'raw' [`Item`] with the ['ItemType'] */
    pub fn make_raw(&self) -> Item {
        self.policy.make_raw(self.self_ref())
    }
    /** returns a reference to self */
    fn self_ref(&self) -> ItemTypeRef {
        self.self_ref.upgrade().expect("bad???")
    }
    /** check whether the Item can be updated using the specified values. It needs to return any errors if any user-supplied value is wrong.
     */
    pub fn check_valid(
        &mut self,
        base: &ItemBaseForSerde,
        vals: &HashMap<String, String>,
        world: &mut World,
    ) -> ActionResponse {
        self.policy.check_valid(base, vals, world)
    }
    /** resolve any conflicts between versions (eg server ls local) */
    pub fn resolve_conflict(
        &self,
        world: &mut World,
        conflict: &Conflict,
        changes: &mut ChangeList,
    ) -> NullResult {
        // self.policy.resolve_conflict(world, conflict, changes)
        trace(&format!("conflict detected {:#?}", &conflict));
        // for now, do something simple
        match &conflict.our {
            None => Ok(()),
            Some(o) => {
                let (oib, ov) = split_data_parts(&o.data)?;
                // let mut os = Simple::new();
                // os.set_from_yaml_basic(ov)?;
                match &conflict.their {
                    None => Ok(()),
                    Some(t) => {
                        let (_tib, tv) = split_data_parts(&t.data)?;
                        match &conflict.ancestor {
                            None => Ok(()),
                            Some(a) => {
                                let (_aib, av) = split_data_parts(&a.data)?;
                                //     let mut ts = Simple::new();
                                //     ts.set_from_yaml_basic(tv)?;
                                //     os.name = merge_strings(&os.name, &ts.name);
                                //     os.text = merge_strings(&os.text, &ts.text);
                                //     trace(&format!("merged to {} and {}", os.name, os.text));
                                //     // let item_type_rcrc = world.get_item_type(oib.type_name.clone())?;
                                //     //    let item = Item:: from_parts(oib, item_type_rcrc  , Box::new(os));
                                //     //    let merged_yaml = item.to_yaml()?;
                                //     //    let change = taipo_git_control::Change::new(
                                //     //        taipo_git_control::ObjectOperation::Modify(String::from_utf8_lossy(  &merged_yaml).to_string()),
                                //     //        o.path.clone(),
                                //     //        "resolve conflict".to_owned(),
                                //     //    );
                                //     //    changes.push(change);
                                // }
                                let ib = self.policy.resolve_conflict_both(world, av, ov, tv)?;
                                Item::change_using_parts(
                                    &oib.type_name,
                                    &oib,
                                    ib,
                                    &o.path,
                                    changes,
                                    world,
                                )?;
                                Ok(())
                            }
                        }
                    }
                }
            }
        }
    }
}
/** ref counted reference to an [`ItemType`] */
pub type ItemTypeRef = Rc<RefCell<ItemType>>;
/** code specific to different kinds of item types */
pub trait ItemTypePolicy: Debug {
    /** the kind of the item */
    fn kind(&self) -> ItemKind;
    /** make a 'raw' Item of that ItemType */
    fn make_raw(&self, item_type: ItemTypeRef) -> Item;
    // /** generate changes to resolve a merge conflict */
    // fn resolve_conflict(
    //     &self,
    //     world: &mut World,
    //     conflict: &Conflict,
    //     changes: &mut ChangeList,
    // ) -> NullResult;
    /** generate changes to resolve a merge conflict where both version have the item */
    fn resolve_conflict_both(
        &self,
        world: &mut World,
        ancestor: Value,
        ours: Value,
        theirs: Value,
    ) -> FLResult<Box<dyn ItemData>>;
    /** check whether the Item can be updated using the specified values. It needs to return any errors if any user-supplied value is wrong.
     */
    fn check_valid(
        &mut self,
        base: &ItemBaseForSerde,
        vals: &HashMap<String, String>,
        world: &mut World,
    ) -> ActionResponse;
}
/** simple enum, each [`ItemKind`] has an [`ItemType`]*/
#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum ItemKind {
    Simple,
    Task,
}
impl fmt::Display for ItemKind {
    /** display an ItemType for debugging */
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
/** a collection of [`ItemType`]s */
pub struct ItemTypeRegistry {
    coll: HashMap<ItemKind, ItemTypeRef>,
}
impl ItemTypeRegistry {
    /** create an new registry */
    pub fn new() -> Self {
        trace("making item type registry");
        Self {
            coll: HashMap::new(),
        }
    }
    /** register an [ItemType]*/
    pub fn register(&mut self, item_type: ItemTypeRef) {
        trace1(
            "registering item type",
            &(item_type.deref().borrow().kind()).to_string(),
        );
        if self.coll.contains_key(&item_type.deref().borrow().kind()) {
            panic!(format!(
                "duplicate item type key: {:?}",
                &item_type.deref().borrow().kind()
            ));
        }
        self.coll
            .insert(item_type.deref().borrow().kind(), item_type.clone());
    }
    /** retrieve an [ItemType] */
    pub fn get(&self, kind: ItemKind) -> crate::shared::FLResult<ItemTypeRef> {
        if !self.coll.contains_key(&kind) {
            panic!(format!("no item type for key: {:?}", kind));
        }
        Ok(self
            .coll
            .get(&kind)
            .ok_or_else(|| fanling_error!("item type not found"))?
            .clone())
    }
}
impl Drop for ItemTypeRegistry {
    /** display a trace message when dropping the item type registry */
    fn drop(&mut self) {
        trace("dropping item type registry");
    }
}
/** a [`ItemListEntry`] is an entry in a list of items */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemListEntry {
    // /** ident of the item    pub ident: Ident,
    // /** type name  of the item  pub type_name: Ident,
    /** link to the item */
    pub link: ItemLinkForSerde,
    /** description of the item */
    pub descr: String,
    /** whether item is selected (eg for HTML) */
    pub selected: bool,
    /** special entry such as null link */
    pub special: bool,
    /** level in hierarchy */
    pub level: i8,
    /** shift in level before (HTML) */
    pub level_shift_before: String,
    /** is this a parent (non-leaf) node (for HTML) */
    pub is_parent: bool,
}
impl ItemListEntry {
    /** make the ItemListEntry "special" */
    pub fn set_special(&mut self) {
        self.special = true;
    }
    /** create a "special" ItemListEntry */
    pub fn make_special(d: &str) -> Self {
        Self {
            descr: d.to_owned(),
            special: true,
            ..Default::default()
        }
    }
    /** convert from an [`Item`] */
    pub fn from_item(item: &Item) -> FLResult<Self> {
        let il = ItemLink::from(item);
        Ok(Self {
            link: ItemLinkForSerde::from_link(&il)?,
            descr: item.description_for_list(),
            ..Default::default()
        })
    }
    /** convert from an [`ItemLink`] */
    pub fn from_link(link: &mut ItemLink, world: &mut World) -> FLResult<Self> {
        let item_rcrc = link.resolve_link(world)?;
        let item = item_rcrc.deref().borrow();
        Self::from_item(&item)
    }
}
impl Default for ItemListEntry {
    fn default() -> Self {
        Self {
            link: ItemLinkForSerde::new("".to_owned()),
            descr: "".to_owned(),
            selected: false,
            special: false,
            level: 0,
            level_shift_before: "".to_owned(),
            is_parent: false,
        }
    }
}

impl fmt::Display for ItemListEntry {
    /** format for display -- just use the description */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.descr)
    }
}
/** a selectable list of [`ItemLink`]s */
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemListEntryList {
    /** the entries */
    pub entries: Vec<ItemListEntry>,
    /** HTML to go after to fix level */
    pub final_adjust_level: String,
}
impl ItemListEntryList {
    /** create an ItemListEntryList from a vector  */
    pub fn from_vec(ilev: Vec<ItemListEntry>) -> Self {
        Self {
            entries: ilev,
            final_adjust_level: "".to_string(),
        }
    }
    /** select the item with the specified ident */
    pub fn select(&mut self, ident: Ident) {
        for ile in self.entries.iter_mut() {
            ile.selected = ile.link.ident == ident;
        }
    }
    /** add an entry to the end of the list */
    pub fn add(&mut self, ile: ItemListEntry) {
        self.entries.push(ile);
    }
    /**  add an entry to the start of the list */
    pub fn prepend(&mut self, ile: ItemListEntry) {
        self.entries.insert(0, ile);
    }
    /** calculate level changes for HTML list */
    pub fn set_level_changes(&mut self) {
        let mut prev_level = 0;
        for ile in self.entries.iter_mut().rev() {
            ile.is_parent = prev_level > ile.level;
            prev_level = ile.level;
        }
        let mut prev_level = 0;
        for ile in self.entries.iter_mut() {
            ile.level_shift_before = if ile.level > prev_level {
                "<ul class=nested>".repeat((ile.level - prev_level).try_into().expect("bad???"))
            } else if ile.level == prev_level {
                "".to_owned()
            } else {
                "</ul>".repeat((prev_level - ile.level).try_into().expect("bad???"))
            };
            prev_level = ile.level;
            self.final_adjust_level = "</ul>".repeat(ile.level.try_into().expect("bad???"));
        }
    }
    /** whether list has any entries */
    pub fn has_entries(&self) -> bool {
        !self.entries.is_empty()
    }
    /** create from a list of [`ItemLink`]s */
    pub fn from_links(links: &mut Vec<ItemLink>, world: &mut World) -> Self {
        Self::from_vec(
            links
                .iter_mut()
                .map(|l| ItemListEntry::from_link(l, world).unwrap())
                .collect(),
        )
    }
}
/** an identifier (actually a String) */
pub type Ident = String;
/** a Rust pointer to an [`Item`] */
pub type ItemRef = Rc<RefCell<Item>>;
/** a Rust weak pointer to an [`Item`] */
#[derive(Clone)]
pub struct ItemWeakRef {
    wkref: Weak<RefCell<Item>>,
}
impl ItemWeakRef {
    /** upgrade to a real pointer */
    pub fn to_rcrc(&self) -> FLResult<ItemRef> {
        Ok(self
            .wkref
            .upgrade()
            .ok_or_else(|| fanling_error!("invalid weak pointer"))?)
    }
    // pub fn to_ref(&self) -> FLResult<&Item> {
    //     Ok(&self.to_rcrc()?.deref().borrow())
    // }
    // pub fn to_mut(&self) -> FLResult<&mut Item> {
    //     Ok(&mut self.to_rcrc()?.deref().borrow_mut())
    // }
}
impl From<&ItemRef> for ItemWeakRef {
    /** downgrade an ItemRef to a weak pointer */
    fn from(ir: &ItemRef) -> Self {
        ItemWeakRef {
            wkref: Rc::downgrade(ir),
        }
    }
}
impl fmt::Debug for ItemWeakRef {
    /** display for debugging */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(ItemWeakRef: {})",
            if self.wkref.upgrade().is_some() {
                "ok"
            } else {
                "none"
            }
        )
    }
}

/** data about an item that can be displayed in a list */
#[derive(Clone, Debug)]
pub enum ItemLinkData {
    Unresolved(Ident),
    Resolved(ItemWeakRef),
}
/** points to an [`Item`], Can be 'resolved' (looked up)  */
#[derive(Clone, Debug)]
pub struct ItemLink {
    data: ItemLinkData,
}
impl ItemLink {
    /** create a new [`ItemLink`]  */
    pub fn new(ident: Ident) -> Self {
        Self {
            data: ItemLinkData::Unresolved(ident),
        }
    }
    /** get the ident that the [`ItemLink`] is linked to */
    pub fn ident(&self) -> FLResult<Ident> {
        match &self.data {
            ItemLinkData::Unresolved(id) => Ok(id.to_string()),
            ItemLinkData::Resolved(ir) => Ok(ir.to_rcrc()?.deref().borrow().ident()),
        }
    }
    /** resolves an [`ItemLink`] to point to an [`Item`] */
    pub fn resolve_link(&mut self, world: &mut World) -> FLResult<ItemRef> {
        match &self.data {
            ItemLinkData::Unresolved(id) => {
                let ir = world.get_item(id.to_string())?;
                self.data = ItemLinkData::Resolved(ItemWeakRef::from(&ir));
                Ok(ir)
            }
            ItemLinkData::Resolved(ir) => ir.to_rcrc(),
        }
    }
}

impl From<ItemLinkForSerde> for ItemLink {
    /** create an ItemLinkForSerde from an ItemLink -- just copy the Ident */
    fn from(il: ItemLinkForSerde) -> Self {
        Self::new(il.ident)
    }
}
impl From<ItemRef> for ItemLink {
    /** create  an ItemLink from an ItemRef --  just copy the IdentRef */
    fn from(item_ref: ItemRef) -> Self {
        Self {
            data: ItemLinkData::Resolved(ItemWeakRef::from(&item_ref)),
        }
    }
}
impl From<Ident> for ItemLink {
    /** make an item link from an item */
    fn from(ident: Ident) -> Self {
        ItemLink::new(ident)
    }
}
impl From<&Item> for ItemLink {
    /** create an ItemLink from an ItemLink -- just copy the Ident  */
    fn from(item: &Item) -> Self {
        Self::new(item.ident())
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
/** an [ItemLink] in a form suitable for (de)serialisation */
pub struct ItemLinkForSerde {
    #[serde(flatten)]
    /** the identifier as a string */
    pub ident: String,
}
impl ItemLinkForSerde {
    /** make a ItemLinkForSerde */
    pub fn new(ident: Ident) -> Self {
        Self { ident }
    }
    /** conveert an `[ItemLink]` to string for serialisation */
    fn from_link(il: &ItemLink) -> FLResult<Self> {
        Ok(Self { ident: il.ident()? })
    }
}
/** for debugging trace */
fn trace1(m: &str, p: &str) {
    println!(
        "item {}",
        Colour::Green
            .on(Colour::Black)
            .paint(format!("{}: {}", m, p))
    );
}

/**template data for modifying base field */
pub struct NewBaseTemplate {
    pub next_op: String,
    pub next_op_name: String,
    pub has_ident: bool,
    pub ident: String,
    pub parent: ItemListEntryList,
    pub can_be_parent: bool,
    pub sort: String,
    pub can_be_context: bool,
}
impl NewBaseTemplate {
    pub fn from_base(
        base: &mut ItemBase,
        is_for_update: bool,
        world: &mut World,
    ) -> FLResult<Self> {
        let parents = base.get_parents(world)?;
        let op_name = if is_for_update { "Update" } else { "Create" };
        let ident = if is_for_update {
            base.get_ident()
        } else {
            "".to_owned()
        };
        Ok(NewBaseTemplate {
            next_op: op_name.to_string(),
            next_op_name: op_name.to_string(),
            has_ident: is_for_update,
            ident,
            parent: parents,
            can_be_parent: base.can_be_parent(),
            sort: base.get_sort(),
            can_be_context: base.can_be_context(),
        })
    }
}
/** template data for showing base fields */
pub struct ShowBaseTemplate {
    pub ident: String,
    pub parent: ItemListEntry,
    pub can_be_parent: bool,
    pub sort: String,
    pub children: ItemListEntryList,
    pub has_children: bool,
    pub can_be_context: bool,
}
impl ShowBaseTemplate {
    /** fill in fields */
    pub fn from_base(base: &mut ItemBase, world: &mut World) -> FLResult<Self> {
        let parent = base.parent_for_display(world)?;
        let children = base.get_item_children(world)?;
        let has_children = !children.entries.is_empty();
        Ok(Self {
            ident: base.get_ident(),
            parent,
            can_be_parent: base.can_be_parent(),
            sort: base.get_sort(),
            children,
            has_children,
            can_be_context: base.can_be_context(),
        })
    }
}

/** for debugging trace */
fn trace(m: &str) {
    println!("item {}", Colour::Fixed(12).on(Colour::Fixed(233)).paint(m));
}

#[cfg(test)]
mod tests {
    #[test]
    fn item_type_registry() -> crate::shared::NullResult {
        let mut reg = super::ItemTypeRegistry::new();
        let item_type = super::ItemType::new(Box::new(crate::simple::SimpleTypePolicy {}));
        reg.register(item_type);
        let _item_type2 = reg.get(super::ItemKind::Simple)?;
        // TODO code me:   assert_eq!(item_type.clone().borrow().mark, item_type2.borrow().mark); // make sense?
        Ok(())
    }
}
