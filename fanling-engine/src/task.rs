/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! implements [`Task`] items */
use crate::item::{
    Ident, Item, ItemBase, ItemBaseForSerde, ItemData, ItemLink, ItemListEntry, ItemListEntryList,
    NewBaseTemplate, ShowBaseTemplate,
};
use crate::markdown;
use crate::shared::{FLResult, FanlingError, NullResult};
//#[macro_use]
use crate::fanling_error;
use crate::world::{ActionResponse, World};
use ansi_term::Colour;
use askama::Template;
use chrono::offset::TimeZone;
use chrono::{NaiveDateTime, Utc};
use log::trace;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use std::boxed::Box;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::ops::Deref;
use taipo_git_control::ChangeList;
use taipo_git_control::Conflict;
/** possible values for the status field */
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum TaskStatus {
    Open,
    Closed,
}
impl TaskStatus {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<TaskStatus, D::Error> {
        let status: String = Deserialize::deserialize(deserializer)?;
        let ts = match status.to_lowercase().as_str() {
            "open" | "" => TaskStatus::Open,
            "closed" => TaskStatus::Closed,
            _ => {
                trace(&format!("unknown status: {}", status));
                TaskStatus::Open
            }
        };
        Ok(ts)
    }
}
impl fmt::Display for TaskStatus {
    /** */
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Default for TaskStatus {
    fn default() -> Self {
        Self::Open
    }
}
/** data for a task item , a task item, like a wiki page */
#[derive(Debug, Clone)]
pub struct Task {
    /** the name of the page */
    name: String,
    /** the text of the page in MarkDown format */
    text: String,
    /** */
    context: Option<ItemLink>,
    /** status of the task */
    status: TaskStatus,
    /** priority of the task */
    priority: i8,
    /** when the task was closed (if it has been) */
    when_closed: chrono::NaiveDateTime,
    project: String,
    /** */
    deadline: chrono::NaiveDateTime,
    /** */
    show_after_date: chrono::NaiveDateTime,
    /** */
    blockedby: Vec<ItemLink>,
}
impl Task {
    /** create a new [Task]  */
    pub fn new() -> Self {
        Self {
            name: "".to_owned(),
            text: "".to_owned(),
            context: None,
            priority: 10,
            status: TaskStatus::Open,
            when_closed: NaiveDateTime::from_timestamp(0, 0),
            project: "".to_owned(),
            deadline: NaiveDateTime::from_timestamp(0, 0),
            show_after_date: NaiveDateTime::from_timestamp(0, 0),
            blockedby: vec![],
        }
    }
    // pub fn set_context(&mut self, context: ItemLink) {
    //     self.context = Some(context);
    // }
    /**  data that can be used to display contexts for selection in a template*/
    pub fn get_contexts(&mut self, world: &mut World) -> FLResult<ItemListEntryList> {
        let mut contexts = world.search_contexts()?;
        //  contexts.prepend(ItemListEntry::make_special("no context"));
        // let match_ident = match self.resolve_context(world)? {
        //     Some(item) => item.deref().borrow().ident(),
        //     _ => "".to_owned(), /* match the no-context option */
        // };
        if let Some(c) = &mut self.context {
            let context_item = world.resolve_link(c)?;
            contexts.select(context_item.deref().borrow().ident());
        }
        Ok(contexts)
    }
    /** data that can be used to display the context in a template */
    pub fn context_for_display(&mut self, world: &mut World) -> FLResult<ItemListEntry> {
        // let context_item = world.get_item(self.context)?;
        let context_item = world.resolve_link(self.context.as_mut().unwrap())?;
        let context = context_item.deref().borrow();
        Ok(ItemListEntry::from_item(&context)?)
        // let p = self.resolve_context(world)?;
        // let context = match p {
        //     Some(p) => {
        //         let item: &Item = &p.deref().borrow();
        //         ItemListEntry::from_item(item)?
        //     }
        //     None => ItemListEntry::make_special("no context"),
        // };
        // Ok(context)
    }
    fn task_from(task: &mut TaskForSerde, _world: &mut World) -> FLResult<Self> {
        let context1 = task.context.clone();
        let context2 = if context1 == "" {
            "default_context".to_owned()
        } else {
            context1
        };
        trace(&format!("using '{}' for context", context2));
        //   let _context = world.get_item(context2.clone());
        let context_link: ItemLink = ItemLink::from(context2.clone());
        trace(&format!(
            "from {:?} made context link {:?}",
            &task.context, &context_link
        ));
        Ok(Self {
            name: task.name.clone(),
            text: task.text.clone(),
            context: Some(context_link),
            status: task.status.clone(),
            priority: task.priority.clone(),
            when_closed: task.when_closed,
            project: task.project.clone(),
            deadline: task.deadline,
            show_after_date: task.show_after_date,
            blockedby: task
                .blockedby
                .iter()
                .map(|il| ItemLink::from(il.clone()))
                .collect(),
        })
    }
    /** implement the close action */
    fn close(&mut self, _world: &mut World) -> NullResult {
        self.status = TaskStatus::Closed;
        self.when_closed = Utc::now().naive_utc();
        Ok(())
    }
    /** implement the re-open action */
    fn reopen(&mut self, _world: &mut World) -> NullResult {
        self.status = TaskStatus::Open;
        // self.when_closed = Utc::now().naive_utc();
        Ok(())
    }
}
impl crate::item::ItemData for Task {
    fn for_edit(
        &mut self,
        base: &mut ItemBase,
        is_for_update: bool,
        world: &mut World,
    ) -> fanling_interface::ResponseResult {
        let broken_text = self.text.replace("\n", "&#10;");
        trace(&format!("{} converted to {}", self.text, broken_text));
        let contexts = self.get_contexts(world)?;
        let nt = NewTaskTemplate {
            data: &self,
            base: NewBaseTemplate::from_base(base, is_for_update, world)?,
            broken_text,
            status: self.status,
            priority: self.priority,
            context: contexts,
            when_closed: self.when_closed,
            deadline: self.deadline,
            show_after_date: self.show_after_date,
        };
        let mut resp = fanling_interface::Response::new();
        resp.clear_errors(vec![
            "name-error".to_owned(),
            "priority-error".to_owned(),
            "show-after-date-error".to_owned(),
            "".to_owned(),
        ]);
        resp.add_tag("content", &(nt.render()?));
        trace(&format!("for edit {:?}", &resp));
        Ok(resp)
    }
    fn for_show(
        &mut self,
        base: &mut ItemBase,
        world: &mut World,
    ) -> fanling_interface::ResponseResult {
        let t = ShowTaskTemplate {
            name: self.name.clone(),
            rendered_text: markdown::render(&self.text),
            base: ShowBaseTemplate::from_base(base, world)?,
            status: self.status,
            priority: self.priority,
            can_be_context: false,
            context: self.context_for_display(world)?,
            when_closed: self.when_closed.format("%Y-%m-%d").to_string(),
            deadline: self.deadline,
            show_after_date: self.show_after_date,
        };
        let mut resp = fanling_interface::Response::new();
        resp.add_tag("content", &(t.render()?));
        trace(&format!("for show {:?}", &resp));
        Ok(resp)
    }
    fn to_yaml(&self, base: &crate::item::ItemBase) -> Result<Vec<u8>, FanlingError> {
        let for_serde = TaskItemForSerde {
            base: crate::item::ItemBaseForSerde::from_base(base)?,
            data: TaskForSerde::try_from(self)?,
        };
        let yaml = serde_yaml::to_vec(&for_serde)?;
        trace(&format!("yaml is {}", String::from_utf8_lossy(&yaml)));
        Ok(yaml)
    }
    fn is_open(&self) -> bool {
        match self.status {
            TaskStatus::Open => true,
            TaskStatus::Closed => false,
        }
    }
    fn is_ready(&self) -> bool {
        if !self.is_open() {
            return false;
        }
        let now = Utc::now().naive_utc();
        if self.show_after_date != NaiveDateTime::from_timestamp(0, 0)
            && self.show_after_date.cmp(&now) == Ordering::Greater
        {
            return false;
        }
        // TODO finish coding (blocking)
        true
    }
    /** an English-language description */
    fn description(&self) -> String {
        let mut parts: Vec<String> = vec!["→ ".to_owned()];
        match self.status {
            TaskStatus::Open => {}
            _ => parts.push(format!("[{:?}] ", self.status)),
        }
        if self.deadline >= NaiveDateTime::from_timestamp(1, 0) {
            parts.push(format!("[{}] ", self.deadline.format("%Y-%m-%d")));
        }
        parts.push(self.name.clone());
        parts.join("")
    }
    // /** a description that can be used in a list */
    // fn description_for_list(&self) -> String {
    //     self.name.clone()
    // }
    /** this can assume that all data is ok, or just return error */
    fn set_data(&mut self, vals: &HashMap<String, String>, world: &mut World) -> NullResult {
        match vals.get("name") {
            Some(s) => self.name = s.to_string(),
            _ => return Err(fanling_error!("no name")),
        };
        self.text = match vals.get("text") {
            Some(s) => s.to_string(),
            _ => "".to_owned(),
        };
        self.priority = match vals.get("priority") {
            Some(s) => s.parse::<i8>()?,
            _ => 0,
        };
        self.context = match vals.get("context") {
            Some(c) => {
                let context_link: ItemLink = ItemLink::from(world.get_item(c.to_string())?);
                trace(&format!(
                    "from {:?} made context link {:?} in set data",
                    &c, &context_link
                ));
                Some(context_link)
            }
            _ => None,
        };
        self.deadline = match vals.get("deadline") {
            Some(dl) => Utc.datetime_from_str(dl, "%F %T")?.naive_utc(),
            _ => return Err(fanling_error!("unvalidated bad date")),
        };
        self.show_after_date = match vals.get("show_after_date") {
            Some(dl) => Utc.datetime_from_str(dl, "%F %T")?.naive_utc(),
            _ => return Err(fanling_error!("unvalidated bad date")),
        };
        Ok(())
    }
    fn try_update(
        &mut self,
        _base: &ItemBaseForSerde,
        vals: &HashMap<String, String>,
        _world: &mut World,
    ) -> ActionResponse {
        let mut ar = ActionResponse::new();
        ar.assert(
            !vals["name"].is_empty(),
            "name-error",
            "Name must be non-blank.",
        );
        ar.assert(
            vals["priority"].parse::<i8>().is_ok(),
            "priority-error",
            "Priority must be numeric",
        );
        /* TODO: validate context */
        ar.assert(
            Utc.datetime_from_str(&vals["deadline"], "%F %T").is_ok(),
            "deadline-error",
            "Invalid deadline date",
        );
        ar.assert(
            Utc.datetime_from_str(&vals["show_after_date"], "%F %T")
                .is_ok(),
            "show-after-date-error",
            "Invalid show-after date",
        );
        ar
    }
    fn set_from_yaml(&mut self, yaml: serde_yaml::Value, world: &mut World) -> NullResult {
        //    *self = serde_yaml::from_value(yaml)?;
        trace("setting task from yaml...");
        let mut tfs = TaskForSerde::default();
        tfs.set_from_yaml(yaml)?;
        *self = Task::task_from(&mut tfs, world)?;
        trace("set task from yaml.");
        Ok(())
    }
    fn do_action(
        &mut self,
        base: &mut ItemBase,
        action: crate::Action,
        //    _json_value: serde_json::value::Value,
        world: &mut World,
    ) -> fanling_interface::ResponseResult {
        match &action {
            // TODO block and unblock
            crate::Action::Close => {
                self.close(world)?;
                Ok(self.for_show(base, world)?)
            }
            crate::Action::Reopen => {
                self.reopen(world)?;
                Ok(self.for_show(base, world)?)
            }
            _ => panic!("invalid action {:?}", action),
        }
    }
    /** copy from another item data */
    fn fanling_clone(&self) -> FLResult<Box<dyn ItemData>> {
        Ok(Box::new(Self {
            name: self.name.clone(),
            text: self.text.clone(),
            context: None,
            priority: self.priority,
            status: TaskStatus::Open,
            when_closed: NaiveDateTime::from_timestamp(0, 0),
            project: self.project.clone(),
            deadline: NaiveDateTime::from_timestamp(0, 0),
            show_after_date: NaiveDateTime::from_timestamp(0, 0),
            blockedby: vec![],
        }))
    }
}
/** task item in a form that can be serialised */
#[derive(Serialize, Deserialize)]
struct TaskItemForSerde {
    #[serde(flatten)]
    base: crate::item::ItemBaseForSerde,
    #[serde(flatten)]
    data: TaskForSerde,
}
impl TaskItemForSerde {
    //     fn set_from_yaml(&mut self, yaml: serde_yaml::Value) -> NullResult {
    //         *self = serde_yaml::from_value(yaml)?;
    //         Ok(())
    //     }
}
/** task data in a form that can be serialised */
#[derive(Serialize, Deserialize)]
struct TaskForSerde {
    /** the name of the page */
    #[serde(default)]
    #[serde(skip_serializing_if = "std::string::String::is_empty")]
    #[serde(alias = "heading")]
    name: String,
    /** the text of the page in MarkDown format */
    #[serde(default)]
    #[serde(skip_serializing_if = "std::string::String::is_empty")]
    text: String,
    /** */
    //   #[serde(default)]
    context: Ident,
    /** */
    #[serde(default)]
    #[serde(skip_serializing_if = "TaskStatus::is_default")]
    #[serde(deserialize_with = "TaskStatus::deserialize")]
    status: TaskStatus,
    /** */
    #[serde(default)]
    priority: i8,
    /** when the task was closed (if it has been) */
    #[serde(alias = "whenclosed")]
    #[serde(deserialize_with = "ItemBaseForSerde::deserialize")]
    when_closed: chrono::NaiveDateTime,
    /**  old field from legacy daata */
    #[serde(default)]
    #[serde(skip_serializing_if = "std::string::String::is_empty")]
    project: String,
    /** */
    #[serde(deserialize_with = "ItemBaseForSerde::deserialize")]
    deadline: chrono::NaiveDateTime,
    /** */
    #[serde(alias = "showafterdate")]
    #[serde(deserialize_with = "ItemBaseForSerde::deserialize")]
    show_after_date: chrono::NaiveDateTime,
    /** */
    #[serde(alias = "waitingon")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    blockedby: Vec<Ident>,
    /** old field from legacy daata */
    #[serde(default)]
    closed: bool,
}
impl Default for TaskForSerde {
    fn default() -> Self {
        Self {
            name: "".to_owned(),
            text: "".to_owned(),
            context: "".to_owned(),
            status: TaskStatus::Open,
            priority: 0,
            when_closed: NaiveDateTime::from_timestamp(0, 0),
            project: "".to_owned(),
            deadline: NaiveDateTime::from_timestamp(0, 0),
            show_after_date: NaiveDateTime::from_timestamp(0, 0),
            blockedby: vec![],
            closed: false,
        }
    }
}
impl TaskForSerde {
    fn set_from_yaml(&mut self, yaml: serde_yaml::Value) -> NullResult {
        *self = serde_yaml::from_value(yaml)?;
        self.fix_data();
        Ok(())
    }
    fn fix_data(&mut self) {
        if self.closed {
            self.status = TaskStatus::Closed;
        }
        // TODO project -> parent
    }
}
impl TryFrom<&Task> for TaskForSerde {
    type Error = FanlingError;
    fn try_from(task: &Task) -> FLResult<Self> {
        Ok(Self {
            name: task.name.clone(),
            text: task.text.clone(),
            context: task.context.as_ref().unwrap().ident()?,
            status: task.status.clone(),
            priority: task.priority.clone(),
            when_closed: task.when_closed,
            project: task.project.clone(),
            deadline: task.deadline,
            show_after_date: task.show_after_date,
            blockedby: task
                .blockedby
                .iter()
                .map(|il| il.ident().unwrap())
                .collect(),
            closed: false,
        })
    }
}
/** template data for creating a new task item */
#[derive(Template)]
#[template(path = "new-task.html")]
struct NewTaskTemplate<'a> {
    pub data: &'a Task,
    pub base: NewBaseTemplate,
    pub broken_text: String,
    pub status: TaskStatus,
    pub priority: i8,
    pub context: ItemListEntryList,
    pub when_closed: NaiveDateTime,
    pub deadline: chrono::NaiveDateTime,
    pub show_after_date: chrono::NaiveDateTime,
}

/** template data for showing a task item */
#[derive(Template)]
#[template(path = "show-task.html")]
struct ShowTaskTemplate {
    pub name: String,
    pub rendered_text: String,
    pub base: ShowBaseTemplate,
    pub status: TaskStatus,
    pub priority: i8,
    pub context: ItemListEntry,
    pub can_be_context: bool,
    pub when_closed: String,
    pub deadline: chrono::NaiveDateTime,
    pub show_after_date: chrono::NaiveDateTime,
}

/** policy for the task item type*/
#[derive(Debug)]
pub struct TaskTypePolicy {}
impl TaskTypePolicy {
    pub fn new() -> Self {
        Self {}
    }
    pub fn new_boxed() -> Box<Self> {
        Box::new(Self::new())
    }
}
impl crate::item::ItemTypePolicy for TaskTypePolicy {
    fn kind(&self) -> crate::item::ItemKind {
        crate::item::ItemKind::Task
    }
    fn make_raw(&self, item_type: crate::item::ItemTypeRef) -> Item {
        let item = Item::new_with_data(item_type, Box::new(Task::new()));
        item
    }
    fn resolve_conflict(&self, conflict: &Conflict, _changes: &mut ChangeList) -> NullResult {
        trace(&format!("conflict detected {:#?}", &conflict));
        unimplemented!() /* resolve conflict TODO */
    }
}

/** convenience function for debug traces */
fn trace(m: &str) {
    trace!("{}", m);
    println!("task {}", Colour::Black.on(Colour::Fixed(229)).paint(m));
}
