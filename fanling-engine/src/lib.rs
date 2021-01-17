/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*!

The `fanling_engine` crate implements the engine of the Fanling
application (the common code across platforms).

## Fanling

Fanling is a distributed note-taking system that is currently implemented on:

* Linux PC
* Android

It should also be possible to build Fanling on Microsoft Windows and
Apple PC platforms. It should also be possible to write an iPhone port
of the Android version.

## The Fanling Engine

Most of the functionality of Fanling is contained in the Fanling
Engine, which is architecture-independent and shared between the
architecture-specific main programs.

The Fanling engine implements the [`fanling_interface::Engine`] trait,
which is used by the platform-specific implementations.

The engine contains the following modules:

* `item` -- implements a single item (page, node)
* `markdown` -- supports markdown formatting
* `search` -- searches for items (uses sqlite)
* `shared` -- some shared code used in multiple modules
* `simple` -- implements the 'simple' item type (in effect, a wiki page)
* `store` -- stores items (using Git)
* `task` --  implements the 'task' item type (a to-do item)
* `world` -- the collection of all items

*/
#[warn(missing_docs, unreachable_pub, unused_extern_crates, unused_results)]
#[deny(const_err, unused_must_use)]
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate quick_error;
use crate::world::WorldStatus;
use askama;

use fanling_interface;

pub use taipo_git_control;

//use std::panic::catch_unwind;
mod item;
mod markdown;
mod search;
mod shared;
mod simple;
mod store;
mod task;
mod world;
use crate::item::ItemBaseForSerde;
pub use crate::shared::{FLResult, FanlingError, NullResult, Tracer};
use fanling_interface::error_response_result;
use log::trace;
pub use search::SearchOptions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::panic;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::time::SystemTime;

// #[macro_use]
// extern crate diesel_migrations;

/** actions that can be requested from the user interface */
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Action {
    Start,
    Shutdown,
    DeleteEverything,
    // SetOptions(Options),
    Pull,
    PushAndQuit { force: bool },
    Push { force: bool },
    Show,
    Edit,
    Update(ItemBaseForSerde, HashMap<String, String>),
    Delete,
    Archive,
    //   Search, /* needs search criteria */
    ListReady,
    ListOpen,
    ListAll,
    New,
    NewChild(item::Ident),
    Create(ItemBaseForSerde, HashMap<String, String>),
    Clone,
    Unknown, /* error */
    /* actions for specific item types */
    Close,
    Reopen,
    GetAll,
    CheckData,
    BlockBy(item::Ident),
    UnblockBy(item::Ident),
    TestError1,
    TestError2,
}
impl Action {
    fn kind(&self) -> ActionKind {
        match self {
            Action::Shutdown
            | Action::PushAndQuit { force: _ }
            | Action::DeleteEverything
            | Action::TestError1 => ActionKind::Engine,
            Action::Start
            | Action::Pull
            | Action::Create(_, _)
            | Action::Update(_, _)
            | Action::ListReady
            | Action::ListOpen
            | Action::ListAll
            | Action::Delete
            | Action::GetAll
            | Action::CheckData
            | Action::Push { force: _ }
            | Action::New
            | Action::Clone
            | Action::NewChild(_)
            | Action::TestError2 => ActionKind::World,
            Action::Show
            | Action::Edit
            | Action::Archive
            | Action::Close
            | Action::Reopen
            | Action::BlockBy(_)
            | Action::UnblockBy(_) => ActionKind::Item,
            Action::Unknown => panic!("unknown action".to_string()),
        }
    }
}

impl Default for Action {
    fn default() -> Self {
        Action::Unknown
    }
}
/** overall classification of the kind of action */
#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum ActionKind {
    Engine,
    World,
    //  ItemType,
    Item,
}
/** Options for various modules */
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Options {
    repo_path: String,
    user_name: String,
    user_email: String,
}

/** contains common fields across requests. The input request is deserialised into a `BasicRequest`. */
#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct BasicRequest {
    //  #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "a")]
    action: Action,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "t")]
    type_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "i")]
    ident: Option<String>,
}
impl BasicRequest {
    pub fn ensure_type_name(&self) -> FLResult<String> {
        let item_kind = self
            .type_name
            .as_ref()
            .ok_or_else(|| fanling_error!("no type name"))?;
        Ok(item_kind.clone())
    }
    pub fn ensure_ident(&self) -> FLResult<String> {
        Ok(self
            .ident
            .as_ref()
            .ok_or_else(|| fanling_error!("no ident"))?
            .clone())
    }
}
// impl Default for BasicRequest {
//     fn default() -> Self {}
// }
/** options for an `Engine`. Some fields are passed down to components. */
#[derive(Debug)]
pub struct EngineOptions {
    /** options have been specified */
    pub correct: bool,
    /** options for the git system */
    pub repo_options: taipo_git_control::RepoOptions,
    /** type of user interface (PC or phone) */
    pub interface_type: InterfaceType,
    /**  options for the search system */
    pub search_options: search::SearchOptions,
    /**  unique prefix for identifiers (must be different for each ser instance) */
    pub uniq_pfx: String,
    /** automatically generate items for missing items in links */
    pub auto_link: bool,
    /** file for checking overall status */
    pub status_path: PathBuf,
    /** path containing all data */
    pub root_path: PathBuf,
}
/** type of user interface that drives this engine. Can be used to elicit different behaviour depending on the interface type. */
#[derive(Copy, Clone, Debug)]
pub enum InterfaceType {
    Android,
    PC,
}

/** the engine of the application (common across platforms) */
pub struct FanlingEngine {
    /** the model */
    world: Option<world::World>,
    /** path containing all data */
    root_path: PathBuf,
    //   /** path containing status */
    //  status_path: PathBuf,
}
impl FanlingEngine {
    /** create a new [FanlingEngine], which implements [fanling_interface::Engine]  */

    pub fn new(opts: &EngineOptions) -> Result<Self, FanlingError> {
        fanling_trace!(&format!(
            "making engine for {:?}, options correct {:?}",
            &opts.interface_type, &opts.correct
        ));
        let status = WorldStatus::load(&opts.status_path);
        trace(&format!("world status is {}", status));
        Ok(Self {
            root_path: opts.root_path.clone(),
            // status_path: opts.status_path.clone(),
            world: if opts.correct && status != WorldStatus::Bad {
                trace(&format!("making world, base is {:?}", &opts.root_path));
                world::World::create_base(&opts.root_path)?;
                WorldStatus::Bad.save(&opts.status_path)?;
                trace("base created");
                let world = world::World::new_and_open(opts)?;
                trace("world exists");
                WorldStatus::Built.save(&opts.status_path)?;
                Some(world)
            } else {
                None
            },
        })
    }
    /** `delete_everything` deletes all the data on disk used by the engine. */
    pub fn delete_everything(&self) -> fanling_interface::ResponseResult {
        std::fs::remove_dir_all(&self.root_path)?;
        fanling_interface::default_response_result()
    }
    fn do_engine_action(
        &mut self,
        basic_request: &BasicRequest,
    ) -> fanling_interface::ResponseResult {
        fanling_trace!("doing engine action");
        match basic_request.action {
            Action::Shutdown => self.shutdown(),
            Action::PushAndQuit { force } => self.push_and_shutdown(force),
            Action::DeleteEverything => self.delete_everything(),
            Action::TestError1 => {
                if let Some(world) = &self.world {
                    trace("making world test error 1");
                    Ok(fanling_interface::Response::new_error_with_tags(&[(
                        "error",
                        &world.test_error()?,
                    )]))
                } else {
                    trace("no world, so no test error 1");
                    Ok(fanling_interface::Response::new())
                }
            }
            _ => error_response_result(&format!("invalid action {:?}", basic_request.action)),
        }
    }
    fn push_and_shutdown(&mut self, force: bool) -> fanling_interface::ResponseResult {
        fanling_trace!("pushing and shutting down");
        if let Some(world) = &mut self.world {
            world.push(force)?;
        }
        self.shutdown()?;
        Ok(fanling_interface::Response::new())
    }
    fn shutdown(&mut self) -> fanling_interface::ResponseResult {
        trace("should shut down now");
        let mut resp = fanling_interface::Response::new();
        resp.set_shutdown_required();
        Ok(resp)
    }
    pub fn touch(&self) {
        trace("touched");
    }
}
impl fanling_interface::Engine for FanlingEngine {
    fn execute(&mut self, body: &str) -> fanling_interface::ResponseResult {
        fanling_trace!(&format!("executing action «{}»", &body));
        let result = panic::catch_unwind(AssertUnwindSafe(move || {
            let now = SystemTime::now();
            let json_body = serde_json::from_str(&body);
            let json_value: serde_json::value::Value = match json_body {
                Err(e) => {
                    let msg = format!("fanling error: {:?}", &e);
                    let re = FanlingError::from(e);
                    re.dump(file!(), line!(), column!());
                    if !cfg!(android) {
                        panic!(msg);
                    } else {
                        serde_json::value::Value::default()
                    }
                }
                Ok(v) => v,
            };
            //dump_fanling_error!(serde_json::from_str(&body));
            fanling_trace!("getting basic request from JSON");
            let basic_request: BasicRequest = serde_json::from_value(json_value.clone())?;
            fanling_trace!(&format!(
                "starting execute action: basic request {:?}, kind {:?}",
                basic_request,
                basic_request.action.kind()
            ));
            let res = match basic_request.action.kind() {
                ActionKind::Engine => self.do_engine_action(&basic_request),
                ActionKind::World | ActionKind::Item => {
                    if let Some(world) = &mut self.world {
                        world.do_action(&basic_request, json_value)
                    } else {
                        Ok(fanling_interface::Response::new())
                    }
                }
            };
            fanling_trace!("action done");
            trace(&format!(
                "execute action done, {:?} took {}s ", //giving {:?}",
                basic_request.action,
                now.elapsed()?.as_millis() as f64 / 1000.0,
                //     &res
            ));
            res
        }));
        match result {
            Ok(res) => res,
            Err(e) => {
                let es = format!("execute error {:?}", e);
                trace(&format!("ee/{}", es)); // does not provide useful info, just "Any"
                Err(Box::new(fanling_error!("error in execute")))
            }
        }
    }
    fn handle_event(
        &mut self,
        event: &fanling_interface::CycleEvent,
    ) -> fanling_interface::TPResult<fanling_interface::Response> {
        fanling_trace!("handling event");
        trace(&format!("handling event {:?}", event));
        match event {
            fanling_interface::CycleEvent::Start// | fanling_interface::CycleEvent::StartPC
                =>
        Ok(fanling_interface::Response::new()),
            fanling_interface::CycleEvent::Pause => { fanling_trace!("pause event");  trace("pause event");  Ok(fanling_interface::Response::new()) /* TODO  activity events */},
            fanling_interface::CycleEvent::Resume =>  { fanling_trace!("resume event"); trace("resume event");  Ok(fanling_interface::Response::new()) /* TODO  activity events */},
            fanling_interface::CycleEvent::Destroy =>  { fanling_trace!("destroy event"); trace("destroy event");  Ok(fanling_interface::Response::new()) /* TODO  activity events */},
            fanling_interface::CycleEvent::Stop |   fanling_interface::CycleEvent::StopPC =>self.push_and_shutdown(false),  //activity events
        }
    }
    fn initial_html(&self) -> fanling_interface::TPResult<String> {
        fanling_trace!("making initial html");
        let now = SystemTime::now();
        let html = if let Some(world) = &self.world {
            world.initial_html()?
        } else {
            "please set the SSH keys and the preferences".to_owned()
        };
        trace(&format!(
            "initial html took {}s",
            now.elapsed()?.as_millis() as f64 / 1000.0
        ));
        Ok(html)
    }
    // fn get_value(&self, key: &str) -> String {
    //     shared::embedded_asset(key)
    // }
    // fn set_callback(&mut self, cb: fn(js: &str)) {
    //     self.interface_callback = Some(cb);
    // }

    /** a description identifying the engine for use in diagnostic
    traces */
    fn trace_descr(&self) -> String {
        match &self.world {
            Some(world) => world.trace_descr(),
            None => "no world".to_owned(),
        }
    }
}
impl Drop for FanlingEngine {
    fn drop(&mut self) {
        trace("dropping engine");
    }
}

/** convenience function for debug traces */
fn trace(txt: &str) {
    trace!("{}", txt);
    println!(
        "engine {}",
        ansi_term::Colour::Yellow
            .on(ansi_term::Colour::White)
            .paint(txt)
    );
}

#[cfg(test)]
mod test;
