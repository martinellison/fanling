/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*!

`fanling-c-interface` provides a C-friendly interface for the Fanling
functionality exposed by the `fanling_interface` crate.

The `fanling-c-interface` interface is wrapped using the SWIG interface
generator to provide a Java interface, which is imported by the `Lowu`
Android application. Together, these provide an Android port of the
Fanling application.

This crate wraps `fanling_interface` in the C-compatible subset of
rust, which is converted by `cbindgen` into C.

The life cycle of the interface is, in general:

* `make_data`
* `execute` and `handle_event` (multiple calls, based on the user's
gestures and the app's life cycle events) -- these return a response
that needs to be followed by the main program
* `delete_data` -- cleans up the data created in `make_data`

*/
//#![cfg(target_os = "android")]
#![allow(non_snake_case)]

// use std::cell::RefCell;
use std::ffi::CStr;
use std::ffi::CString;

use std::os::raw::c_char;
use std::path::PathBuf;
//use taipo_git_control::RepoOptions;

use fanling_engine::{taipo_git_control, EngineOptions, FanlingEngine, InterfaceType};
use fanling_interface::{CycleEvent, Engine};
#[macro_use]
extern crate log;
#[cfg(target_os = "android")]
extern crate android_log;
use libc::c_int;
use serde::Deserialize;
use std::panic;
use std::panic::AssertUnwindSafe;

//#[no_mangle]
/// the main data, including the [`FanlingEngine`] data for the engine

pub struct LowuData {
    is_ok: bool,
    engine: Option<FanlingEngine>,
    last_response: fanling_interface::ResponseResult,
    last_key: CString,
    last_string: CString,
    //  canary: String, // for debug
}
#[no_mangle]
/// return whether it was ok
pub unsafe extern "C" fn is_ok(data: *mut LowuData) -> bool {
    let d = data.as_mut().expect("bad pointer");
    d.is_ok
}
#[no_mangle]
/// get the most recent string
pub unsafe extern "C" fn last_string(data: *mut LowuData) -> *const c_char {
    debug!("getting inital html...");
    let d = data.as_mut().expect("bad pointer");
    d.last_string.as_ptr()
}

//#[no_mangle]
#[repr(u8)]
/// application cycle events to be sent to the engine
pub enum CCycleEvent {
    /// open the application
    Start,
    /// pause the application, engine should save state
    Pause,
    /// resume the application with the state saved as at the most recent `Pause` event
    Resume,
    /// stop the application
    Stop,
    /// stop for PC interface (probably not used)
    StopPC,
    /// destroy the application
    Destroy,
}
impl CCycleEvent {
    pub fn for_c(event: CycleEvent) -> CCycleEvent {
        match event {
            CycleEvent::Start => CCycleEvent::Start,
            CycleEvent::Pause => CCycleEvent::Pause,
            CycleEvent::Resume => CCycleEvent::Resume,
            CycleEvent::Stop | CycleEvent::StopPC => CCycleEvent::Stop,
            CycleEvent::Destroy => CCycleEvent::Destroy,
        }
    }
    pub fn from_c(event: CCycleEvent) -> CycleEvent {
        match event {
            CCycleEvent::Start => CycleEvent::Start,
            CCycleEvent::Pause => CycleEvent::Pause,
            CCycleEvent::Resume => CycleEvent::Resume,
            CCycleEvent::Stop | CCycleEvent::StopPC => CycleEvent::Stop,
            CCycleEvent::Destroy => CycleEvent::Destroy,
        }
    }
}
#[derive(Deserialize)]
struct FanlingOptions {
    pub correct: bool,
    pub path: String,
    pub repo_name: String,
    pub name: String,
    pub email: String,
    pub url: String,
    pub have_url: bool,
    pub branch: String,
    pub unique_prefix: String,
    pub ssh_path: String,
    pub slurp_ssh: bool,
    pub auto_link: bool,
}
#[no_mangle]
/// creates the main data structure. If you call this, you should call `delete_data` at the end of the program. Note that we initialise the android log; we can only do this once but this code is called more than once, and we have no easy way to check whether it has been called already, so we just ignore any error.
pub unsafe extern "C" fn make_data(fanling_options_json_c: *const c_char) -> *mut LowuData {
    let mut is_ok = true;
    //  println!("making data...");
    #[cfg(target_os = "android")]
    match android_log::init("taipo") {
        Err(e) => {
            is_ok = false;
            debug!("on initialising android logger, got error: {:?}", e)
        }
        _ => {
            trace!("android logging started");
        }
    }
    debug!("making engine options in rust...");
    let fanling_options_json = string_from_c(fanling_options_json_c);
    debug!("...deserialising from json: '{}'", fanling_options_json);
    let fanling_options: FanlingOptions = serde_json::from_str(&fanling_options_json)
        .or_else(|err| {
            debug!("bad deserialise: {:?}", err);
            is_ok = false;
            Err(err)
        })
        .expect("bad deserialise");
    trace!("...creating engine options...");
    let engine_options = EngineOptions {
        correct: fanling_options.correct,
        repo_options: taipo_git_control::RepoOptions {
            path: [
                PathBuf::from(fanling_options.path.clone()),
                PathBuf::from(fanling_options.repo_name.clone() + ".git"),
            ]
            .iter()
            .collect::<PathBuf>()
            .into_boxed_path(),
            base_path: PathBuf::from(fanling_options.path.clone()).into_boxed_path(),
            name: fanling_options.name,
            email: fanling_options.email,
            url: if fanling_options.have_url {
                Some(fanling_options.url)
            } else {
                None
            },
            required_branch: Some(fanling_options.branch),
            ssh_path: PathBuf::from(fanling_options.ssh_path).into_boxed_path(),
            slurp_ssh: fanling_options.slurp_ssh,
            ..taipo_git_control::RepoOptions::default()
        },
        interface_type: InterfaceType::Android,
        search_options: fanling_engine::SearchOptions {
            database_path: [
                PathBuf::from(fanling_options.path.clone()),
                PathBuf::from(fanling_options.repo_name.clone() + ".db"),
            ]
            .iter()
            .collect::<PathBuf>()
            .to_string_lossy()
            .to_string(),
        },
        uniq_pfx: fanling_options.unique_prefix,
        auto_link: fanling_options.auto_link,
        status_path: [
            PathBuf::from(fanling_options.path.clone()),
            PathBuf::from(fanling_options.repo_name.clone() + "-status"),
        ]
        .iter()
        .collect::<PathBuf>(),
        root_path: PathBuf::from(fanling_options.path.clone()),
    };
    debug!("options as read {:#?}", engine_options);
    debug!("making data in rust...");
    let mut msg = "Starting engine...".to_owned();
    let mut engine: Option<FanlingEngine> = None;
    debug!("about to create engine");
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        debug!("creating engine...");
        match FanlingEngine::new(&engine_options) {
            Err(e) => {
                error!("could not create engine: {:?}", &e);
                is_ok = false;
                msg = format!("Engine creation error: {:?}", &e);
            }
            Ok(e) => {
                engine = Some(e);
                debug!("engine ok");
                msg = "Engine created".to_string();
            }
        };
    }));
    debug!("engine creation attempted {:?}", result);
    match result {
        Ok(_) => (),
        Err(e) => {
            trace!(
                "making engine failed {:?} which is a {:?}",
                e.as_ref(),
                e.type_id()
            );
            is_ok = false;
            msg = if let Some(x) = e.downcast_ref::<Box<String>>() {
                format!("engine creation error (boxed): {:?}", x)
            } else if let Some(x) = e.downcast_ref::<String>() {
                format!("engine creation error: {:?}", x)
            } else {
                "(not a known type)".to_string()
            };
            trace!("error: {}", msg);
        }
    }
    debug!(
        "got engine ({}: {}), making top level rust data...",
        if is_ok { "OK" } else { "not OK" },
        msg
    );
    let data = Box::into_raw(Box::new(LowuData {
        is_ok,
        engine,
        last_string: string_to_cstring(msg),
        last_key: string_to_cstring("".to_string()),
        last_response: fanling_interface::default_response_result(),
        // canary: "some lowu data".to_string(),
    }));
    debug!("data made in rust.");
    data
}

#[no_mangle]
/// deletes the main data structure
pub unsafe extern "C" fn delete_data(data: *mut LowuData) {
    debug!("deleting rust data...");
    let _b = Box::from_raw(data);
}
fn string_from_c(s: *const c_char) -> String {
    unsafe { CStr::from_ptr(s).to_string_lossy().into_owned() }
}

#[no_mangle]
/// execute an action (just wraps the engine call)
pub extern "C" fn execute(data: *mut LowuData, body: *const c_char) {
    // let bs = CStr::from_ptr(body).to_string_lossy().into_owned();
    let bs = string_from_c(body);
    debug!("executing {} [from rust-c]", bs);
    let mut d = unsafe { data.as_mut().expect("bad pointer") };
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        match &mut d.engine {
            Some(e) => d.last_response = e.execute(&bs),
            None => {
                debug!("no engine!");
            }
        }
        debug!("executed [from rust-c]");
        debug!("execution result {:?} [from rust-c]", d.last_response);
    }));
    match result {
        Ok(_) => (),
        Err(e) => {
            trace!("execution failed {:?}", e);
            d.last_response = Ok(fanling_interface::Response::new_error_with_tags(&[(
                "error",
                &format!("execute error {:?}", &e),
            )]));
        }
    }
}

// #[no_mangle]
// pub extern "C" fn set_options(data: *mut LowuData, options_json: *const c_char) {
//     let opts = string_from_c(options_json);
//     let mut d = unsafe { data.as_mut().expect("bad pointer") };
//     match &mut d.engine {
//         Some(e) => e.set_options(&opts).expect("bad options"),
//         None => {}
//     }
//     d.last_response = fanling_interface::default_response_result();
// }

#[no_mangle]
/// handles a life cycle event (just wraps the engine call)
pub extern "C" fn handle_event(data: *mut LowuData, event: CCycleEvent) {
    let mut d = unsafe { data.as_mut().expect("bad pointer") };
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        debug!("handling event");
        match &mut d.engine {
            Some(e) => {
                d.last_response = e.handle_event(&CCycleEvent::from_c(event));
            }
            None => {}
        }
    }));
    match result {
        Ok(_) => (),
        Err(e) => {
            trace!("handle event failed {:?}", e);
            d.last_response = Ok(fanling_interface::Response::new_error_with_tags(&[(
                "error",
                &format!("life cycle error {:?}", &e),
            )]));
        }
    }
}

#[no_mangle]
/// creates the initial HTML  (just wraps the engine call)
pub extern "C" fn initial_html(data: *mut LowuData) -> *const c_char {
    debug!("getting inital html...");
    let mut d = unsafe { data.as_mut().expect("bad pointer") };
    // debug!("inital html, got lowu data");
    // debug!("inital html, lowu data has '{}'", d.canary); // check d is real

    match &mut d.engine {
        Some(e) => {
            let is = e.initial_html().expect("bad initial html");
            // debug!("inital html, initial string is '{}'", is);
            d.last_string = string_to_cstring(is);
        }
        None => {}
    }
    debug!("inital html, returning...");
    d.last_string.as_ptr()
}

// // #[no_mangle]
// pub extern "C" fn get_value(data: *mut LowuData, key: *const c_char) -> *const c_char {
//     let rkey = string_from_c(key);
//     let mut d = unsafe { data.as_mut().expect("bad pointer") };
//     let cval = d.engine.get_value(&rkey).expect("bad key for value");
//     d.last_string = string_to_cstring(cval);
//     d.last_string.as_ptr()
// }

// #[no_mangle]
// pub extern "C" fn set_callback(data: *mut LowuData, cb: fn()) {
//     d.engine.set_callback(cb);
// }
fn string_to_cstring(s: String) -> CString {
    CString::new(s).expect("string_to_cstring error")
}
#[no_mangle]
/// checks if the response is OK
pub extern "C" fn response_ok(data: *mut LowuData) -> bool {
    let d = unsafe { data.as_ref().expect("bad pointer") };
    d.last_response.is_ok()
}
#[no_mangle]
/// the number of items in the response
pub extern "C" fn response_num_items(data: *mut LowuData) -> c_int {
    let d = unsafe { data.as_ref().expect("bad pointer") };
    match &d.last_response {
        Ok(r) => r.num_tags() as c_int,
        Err(_) => 0,
    }
}
#[no_mangle]
/// a response item, selected by index
pub extern "C" fn response_item(data: *mut LowuData, n: c_int) -> CResponseItem {
    let mut d = unsafe { data.as_mut().expect("bad pointer") };
    let ss = match &d.last_response {
        Ok(r) => r.get_tag(n as usize),
        Err(_e) => ("".to_string(), "".to_string()),
    };
    d.last_key = string_to_cstring(ss.0.to_string());
    d.last_string = string_to_cstring(ss.1);
    CResponseItem {
        key: d.last_key.as_ptr(),
        value: d.last_string.as_ptr(),
    }
}
#[no_mangle]
/// the error message, if the response is an error
pub extern "C" fn response_error(data: *mut LowuData) -> *const c_char {
    let mut d = unsafe { data.as_mut().expect("bad pointer") };
    d.last_string = string_to_cstring(match &d.last_response {
        Ok(_) => "".to_string(),
        Err(e) => e.to_string(),
    });
    d.last_string.as_ptr()
}
#[no_mangle]
/// whether the response requires the application to be shut down
pub extern "C" fn is_shutdown_required(data: *mut LowuData) -> bool {
    let d = unsafe { data.as_ref().expect("bad pointer") };
    match &d.last_response {
        Ok(r) => r.is_shutdown_required(),
        Err(_e) => false,
    }
}
//#[no_mangle]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
/// an item of a response
pub struct CResponseItem {
    pub key: *const c_char,
    pub value: *const c_char,
}
