/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! The fanling_interface crate provides an interface between the engine
 of the Fanling rust application and a main program. This enables the
 same core functionality to be shared across several architectures by
 implementing the same main program functionality for each
 architecture but reusing the common engine.

The main export of this crate is the `Engine` trait. (The
implementation of the `Engine` trait is
[`fanling_engine::FanlingEngine`]).

Currently, this interface is used by:

* a PC implementation [`fanling10::Fanling10`] 
* an Android implementation using the `fanling_c*interface` crate and the
`Lowu` Android app.
*/

use ansi_term;
use std::fmt;

/** trait for an interface between a main program and an engine  */

pub trait Engine {
    /** the engine should carry out the command `body` (in JSON format) and return a response */
    fn execute(&mut self, body: &str) -> ResponseResult;

    // /** send some options to the engine in JSON format. `set_option`
    // should be called before the first `handle_event` call, and again
    // if there is a change to option values. */
    // fn set_options(&mut self, options_json: &str) -> TPResult<()>;
    /** send a life cycle event to the engine */
    fn handle_event(&mut self, event: &CycleEvent) -> TPResult<Response>;
    /** the initial HTML web page for when the app is opened */
    fn initial_html(&self) -> TPResult<String>;
    // /** get a string value from the engine */
    // fn get_value(&self, key: &str) -> String;
    // /** set a callback */
    // fn set_callback(&mut self, cb: fn(js: &str));
}
/// [Result] type for this package
pub type TPResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// either an error or a response, to be sent to the interface
pub type ResponseResult = TPResult<Response>;
/// the default [ResponseResult]
pub fn default_response_result() -> ResponseResult {
    trace("getting default response result");
    Ok(Response::default())
}
/** the response from the Engine resulting from a command or event */
#[derive(Default, Clone, Debug)]
pub struct Response {
    /** An instruction to, for each pair `(tag, html)`, replace the element identified by `tag` with `html`. The order of the pairs will be significant.  */
    tags: Vec<(String, String)>,
    // /** */
    // to_clear: Vec<String>,
    /** set to true to tell the user interface to shut down. The
    engine should have already saved state or whatever it needs to
    do. */
    shutdown_required: bool,
}
impl Response {
    /**  create a response */
    pub fn new() -> Self {
        Response {
            tags: vec![],
            //  to_clear: vec![],
            shutdown_required: false,
        }
    }
    /** */
    pub fn clear_errors(&mut self, errors: Vec<String>) {
        //  self.to_clear = errors.clone();
        for tag in errors {
            self.tags.push((tag.to_owned(), "".to_owned()));
        }
    }
    /** add a tag value pair to the response */
    pub fn add_tag(&mut self, tag: &str, val: &str) {
        self.tags.push((tag.to_owned(), val.to_owned()));
    }
    /** add several tag/value pairs to the response */
    pub fn add_tags(&mut self, tags: &[(&str, &str)]) {
        for ss in tags {
            self.add_tag(ss.0, ss.1)
        }
    }
    /** create a response with  several tag/value pairs */
    pub fn new_with_tags(tags: &[(&str, &str)]) -> Self {
        let mut resp = Self::new();
        resp.add_tags(tags);
        resp
    }
    // /** get the tags to clear from the response */
    // pub fn get_to_clear(&self) -> impl Iterator<Item = &String> {
    //     self.to_clear.iter()
    // }
    /** get the tag/value pairs from the response */
    pub fn get_tags(&self) -> impl Iterator<Item = &(String, String)> {
        self.tags.iter()
    }
    /** get a tag by index */
    pub fn get_tag(&self, i: usize) -> (String, String) {
        self.tags[i].clone()
    }
    /** count of tags */
    pub fn num_tags(&self) -> usize {
        self.tags.len()
    }
    /** should the user interface shut down? */
    pub fn is_shutdown_required(&self) -> bool {
        self.shutdown_required
    }
    /** the user interface should shut down */
    pub fn set_shutdown_required(&mut self) {
        self.shutdown_required = true
    }
}
#[derive(Debug)]
/// another error type
pub struct Error {
    msg: String,
}
impl Error {
    pub fn new(m: &str) -> Self {
        Self { msg: m.to_string() }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for Error {}
/** events in the life cycle of an app. Some of these will only happen on mobile. */
#[derive(Debug)]
pub enum CycleEvent {
    /// open the application
    Start,
    // /// open the application from the PC interface
    // StartPC,
    /// pause the application, engine should save state
    Pause,
    /// restore the application with the state saved as at the most recent `Pause` event
    Resume,
    /// stop the application
    Stop,
    /// stop (from the PC interface)
    StopPC,
}
/** convenience function for debug traces */
pub(crate) fn trace(txt: &str) {
    println!(
        "git {}",
        ansi_term::Colour::Fixed(64)
            .on(ansi_term::Colour::Fixed(192))
            .paint(txt)
    );
}
