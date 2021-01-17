/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! implements [`Task`] items */
use crate::item::{
    Ident, Item, ItemBase, ItemBaseForSerde, ItemData, ItemLink, ItemListEntry, ItemListEntryList,
    NewBaseTemplate, ShowBaseTemplate,
};
use crate::markdown;
use crate::shared::{merge_strings, FLResult, FanlingError, NullResult};
//#[macro_use]
use crate::fanling_error;
use crate::world::{ActionResponse, World};
use ansi_term::Colour;
use askama::Template;
use chrono::offset::TimeZone;
use chrono::{NaiveDateTime, Utc};
use fanling_interface::error_response_result;
use log::trace;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::boxed::Box;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::fmt::Debug;
use std::ops::Deref;

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
    /** display to formatter */
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
    /** the context of the task (the conditions for carrying out the task) */
    context: Option<ItemLink>,
    /** status of the task */
    status: TaskStatus,
    /** priority of the task */
    priority: i8,
    /** when the task was closed (if it has been) */
    when_closed: chrono::NaiveDateTime,
    project: String,
    /** the deadline for completion */
    deadline: chrono::NaiveDateTime,
    /** only show the task as 'ready' after this date */
    show_after_date: chrono::NaiveDateTime,
    /** task is blocked by these other tasks, do not show this task as
    ready until all these tasks are closed */
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
        let context_item = world.resolve_link(
            self.context
                .as_mut()
                .ok_or_else(|| fanling_error!("no context found"))?,
        )?;
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
        let context_link: ItemLink = ItemLink::from(context2);
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
    /** block this task by the task with the `ident` */
    fn block(&mut self, ident: &str) -> NullResult {
        self.blockedby.push(ItemLink::new(ident.to_string()));
        // TODO: checks; prevent duplications
        Ok(())
    }
    /** unblock this task by the task with the `ident` */
    fn unblock(&mut self, ident: &str) -> NullResult {
        self.blockedby.retain(|il| il.ident().unwrap() != ident);
        Ok(())
    }
    #[cfg(test)]
    fn set_test_data(
        &mut self,
        resp: &mut fanling_interface::Response,
        base: &mut ItemBase,
        world: &mut World,
    ) {
        resp.set_test_data("ident", &base.get_ident());
        resp.set_test_data(
            "ready",
            if self.is_ready(world).unwrap() {
                "true"
            } else {
                "false"
            },
        );
        resp.set_test_data("open", if self.is_open() { "true" } else { "false" });
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
        let blockedby = ItemListEntryList::from_links(&mut self.blockedby, world);
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
            blockedby,
        };
        let mut resp = fanling_interface::Response::new();
        resp.clear_errors(vec![
            "name-error".to_owned(),
            "priority-error".to_owned(),
            "show-after-date-error".to_owned(),
            "".to_owned(),
        ]);
        resp.add_tag("content", &(nt.render()?));
        #[cfg(test)]
        {
            self.set_test_data(&mut resp, base, world);
        }
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
            blockedby: ItemListEntryList::from_links(&mut self.blockedby, world),
            potential_blockers: world.search_open_hier()?,
        };
        let mut resp = fanling_interface::Response::new();
        resp.add_tag("content", &(t.render()?));
        #[cfg(test)]
        {
            self.set_test_data(&mut resp, base, world);
        }
        trace(&format!("for show {:?}", &resp));
        Ok(resp)
    }

    fn to_json(&self, base: &crate::item::ItemBase) -> Result<Vec<u8>, FanlingError> {
        let for_serde = TaskItemForSerde {
            base: crate::item::ItemBaseForSerde::from_base(base)?,
            data: TaskForSerde::try_from(self)?,
        };
        let json = serde_json::to_vec(&for_serde)?;
        trace(&format!("json is {}", String::from_utf8_lossy(&json)));
        Ok(json)
    }
    fn is_open(&self) -> bool {
        match self.status {
            TaskStatus::Open => true,
            TaskStatus::Closed => false,
        }
    }
    fn is_ready(&mut self, world: &mut World) -> FLResult<bool> {
        if !self.is_open() {
            trace("task is not ready because not open");
            return Ok(false);
        }
        let now = Utc::now().naive_utc();
        if self.show_after_date != NaiveDateTime::from_timestamp(0, 0)
            && self.show_after_date.cmp(&now) == Ordering::Greater
        {
            trace("task is not ready because before show-after date");
            return Ok(false);
        }
        for bi in &mut self.blockedby {
            let bir = bi.resolve_link(world)?;
            let bi = bir.deref().borrow();
            if bi.is_open() {
                trace(&format!(
                    "task is not ready because blocked by {}",
                    bi.ident()
                ));
                return Ok(false);
            } else {
                trace(&format!(
                    "task is not blocked by {} because not open",
                    bi.ident()
                ));
            }
        }
        trace("task is ready");
        Ok(true)
    }
    /** can be turned into an ident */
    fn descr_for_ident(&self) -> String {
        self.name.clone()
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
    /** a description that can be used in a list */
    fn description_for_list(&self) -> String {
        self.name.clone()
    }
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
                let context_link: ItemLink =
                    ItemLink::from(world.get_item(c.to_string(), "Simple".to_string())?);
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
            _ => NaiveDateTime::from_timestamp(0, 0),
        };
        self.show_after_date = match vals.get("show_after_date") {
            Some(dl) => Utc.datetime_from_str(dl, "%F %T")?.naive_utc(),
            _ => NaiveDateTime::from_timestamp(0, 0),
        };
        Ok(())
    }
    fn set_from_json(&mut self, json: &serde_json::Value, world: &mut World) -> NullResult {
        //    *self = serde_json::from_value(json)?;
        trace("setting task from json...");
        let mut tfs = TaskForSerde::default();
        tfs.set_from_json(json)?;
        *self = Task::task_from(&mut tfs, world)?;
        trace("set task from json.");
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
            crate::Action::Close => {
                self.close(world)?;
                Ok(self.for_show(base, world)?)
            }
            crate::Action::Reopen => {
                self.reopen(world)?;
                Ok(self.for_show(base, world)?)
            }
            crate::Action::BlockBy(ident) => {
                self.block(&ident)?;
                Ok(self.for_show(base, world)?)
            }
            crate::Action::UnblockBy(ident) => {
                self.unblock(&ident)?;
                Ok(self.for_show(base, world)?)
            }
            _ => error_response_result(&format!("invalid action {:?}", action)),
        }
    }
    /** copy from another item data. But status is Open and no blockers. */
    fn fanling_clone(&self) -> FLResult<Box<dyn ItemData>> {
        Ok(Box::new(Self {
            name: self.name.clone(),
            text: self.text.clone(),
            context: self.context.clone(),
            priority: self.priority,
            status: TaskStatus::Open,
            when_closed: NaiveDateTime::from_timestamp(0, 0),
            project: self.project.clone(),
            deadline: self.deadline,
            show_after_date: self.show_after_date,
            blockedby: vec![],
        }))
    }
    /** transitional code to fix some old data */
    fn fix_data(
        &self,
        json: &serde_json::Value,
        base: &mut ItemBase,
        world: &mut World,
    ) -> NullResult {
        //  trace("fixing task data...");
        if let Some(project) = json.get("project") {
            //   trace("task has project");
            if let Some(project_name) = project.as_str() {
                if !base.has_parent() && !project_name.is_empty() {
                    trace(&format!(
                        "no parent and task has project '{}'",
                        &project_name
                    ));
                    let parent_item_rcrc =
                        world.get_item(project_name.to_owned(), "Task".to_owned())?;
                    let parent_item = parent_item_rcrc.deref().borrow();
                    let parent_link = ItemLink::from(&*parent_item);
                    trace(&format!(
                        "setting parent for '{}' to '{}'",
                        base.get_ident(),
                        parent_item.ident(),
                    ));
                    base.set_parent(Some(parent_link));
                    assert_ne!("??", base.get_ident());
                }
            }
        }
        Ok(())
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
    //     fn set_from_json(&mut self, json: serde_json::Value) -> NullResult {
    //         *self = serde_json::from_value(json)?;
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
    /** context of the task */
    //   #[serde(default)]
    context: Ident,
    /** status of the task e.g. open, closed */
    #[serde(default)]
    #[serde(skip_serializing_if = "TaskStatus::is_default")]
    #[serde(deserialize_with = "TaskStatus::deserialize")]
    status: TaskStatus,
    /** priority of the task */
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
    /** deadline for completing the task */
    #[serde(deserialize_with = "ItemBaseForSerde::deserialize")]
    deadline: chrono::NaiveDateTime,
    /** show the task only after this date */
    #[serde(alias = "showafterdate")]
    #[serde(deserialize_with = "ItemBaseForSerde::deserialize")]
    show_after_date: chrono::NaiveDateTime,
    /** task is blocked by these tasks */
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
    /** set from JSON data */
    fn set_from_json(&mut self, json: &serde_json::Value) -> NullResult {
        *self = serde_json::from_value(json.clone())?;
        self.fix_data();
        Ok(())
    }
    /** ensure consistency of the data */
    fn fix_data(&mut self) {
        if self.closed {
            self.status = TaskStatus::Closed;
        }
    }
}
impl TryFrom<&Task> for TaskForSerde {
    type Error = FanlingError;
    fn try_from(task: &Task) -> FLResult<Self> {
        Ok(Self {
            name: task.name.clone(),
            text: task.text.clone(),
            context: task
                .context
                .as_ref()
                .ok_or_else(|| {
                    fanling_error!(&format!("no context found from task: {:#?}", &task))
                })?
                .ident()?,
            status: task.status.clone(),
            priority: task.priority.clone(),
            when_closed: task.when_closed,
            project: task.project.clone(),
            deadline: task.deadline,
            show_after_date: task.show_after_date,
            blockedby: task
                .blockedby
                .iter()
                .map(|il| il.ident().expect("no ident"))
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
    pub blockedby: ItemListEntryList,
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
    pub potential_blockers: ItemListEntryList,
    pub blockedby: ItemListEntryList,
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
    fn resolve_conflict_both(
        &self,
        world: &mut World,
        _ancestor: &Value,
        ours: &Value,
        theirs: &Value,
    ) -> FLResult<Box<dyn ItemData>> {
        let mut ots = TaskForSerde::default();
        ots.set_from_json(ours)?;
        let mut ot = Task::task_from(&mut ots, world)?;
        let mut tts = TaskForSerde::default();
        tts.set_from_json(theirs)?;
        let tt = Task::task_from(&mut tts, world)?;
        ot.name = merge_strings(&ot.name, &tt.name);
        ot.text = merge_strings(&ot.text, &tt.text);
        //   ot.context = "";
        if ot.status == TaskStatus::Open {
            if tt.status == TaskStatus::Closed {
                ot.status = TaskStatus::Closed;
                ot.when_closed = tt.when_closed;
            }
        }
        ot.priority = std::cmp::min(ot.priority, tt.priority);
        //  ot.project = "";
        ot.deadline = std::cmp::min(ot.deadline, tt.deadline);
        ot.show_after_date = std::cmp::min(ot.show_after_date, tt.show_after_date);
        for t in tt.blockedby {
            if !ot.blockedby.contains(&t) {
                ot.blockedby.push(t);
            }
        }
        Ok(Box::new(ot))
    }
    fn check_valid(
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
    /** get item data from serde value */
    fn from_json(&self, values: &Value, world: &mut World) -> FLResult<Box<dyn ItemData>> {
        let mut ts = TaskForSerde::default();
        ts.set_from_json(values)?;
        let t = Task::task_from(&mut ts, world)?;
        Ok(Box::new(t))
    }
}
/** convenience function for debug traces */
fn trace(m: &str) {
    trace!("{}", m);
    println!("task {}", Colour::Black.on(Colour::Fixed(229)).paint(m));
}
