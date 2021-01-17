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
`crate::fanling_engine::FanlingEngine`).

Currently, this interface is used by:

* a PC implementation `fanling10::Fanling10`
* an Android implementation using the `fanling_c*interface` crate and the
`Lowu` Android app.
*/
use std::collections::HashMap;
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
    /** a description identifying the engine for use in diagnostic
    traces */
    fn trace_descr(&self) -> String;
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
///
pub fn error_response_result(msg: &str) -> ResponseResult {
    trace("getting error response result");
    Ok(Response::new_error_with_tags(&vec![("error", msg)]))
}
/** the response from the Engine to the user interface resulting from a command or event */
#[derive(Default, Clone, Debug)]
pub struct Response {
    /** An instruction to, for each pair `(tag, html)`, replace the element identified by `tag` with `html`. The order of the pairs will be significant.  */
    tags: Vec<(String, String)>,
    /** set to true to tell the user interface to shut down. The
    engine should have already saved state or whatever it needs to
    do. */
    shutdown_required: bool,
    /** whether the response includes an error */
    error: bool,
    /** assocated test data if any */
    //   #[cfg(test)]
    test_data: HashMap<String, String>,
}
impl Response {
    /**  create a response */
    pub fn new() -> Self {
        Response {
            tags: vec![],
            //  to_clear: vec![],
            shutdown_required: false,
            error: false,
            //  #[cfg(test)]
            test_data: HashMap::new(),
        }
    }
    /** tell the user interface to clear any errors from the user's display*/
    pub fn clear_errors(&mut self, errors: Vec<String>) {
        //  self.to_clear = errors.clone();
        for tag in errors {
            self.tags.push((tag.to_owned(), "".to_owned()));
        }
        self.error = false;
    }
    /** add a tag value pair to the response */
    pub fn add_tag(&mut self, tag: &str, val: &str) {
        self.tags.push((tag.to_owned(), val.to_owned()));
    }
    /** convenience method */
    pub fn add_error_tag(&mut self, tag: &str, val: &str) {
        self.add_tag(tag, val);
        self.set_error();
    }
    /** add several tag/value pairs to the response */
    pub fn add_tags(&mut self, tags: &[(&str, &str)]) {
        for ss in tags {
            self.add_tag(ss.0, ss.1)
        }
    }
    /** create a response with several tag/value pairs */
    pub fn new_with_tags(tags: &[(&str, &str)]) -> Self {
        let mut resp = Self::new();
        resp.add_tags(tags);
        resp
    }
    /** create a response with several tag/value pairs and an error */
    pub fn new_error_with_tags(tags: &[(&str, &str)]) -> Self {
        let mut resp = Self::new();
        resp.add_tags(tags);
        resp.set_error();
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
        self.shutdown_required = true;
    }
    /**  whether the response includes an error */
    pub fn is_error(&self) -> bool {
        self.error
    }
    /**  set that the response includes an error */
    pub fn set_error(&mut self) {
        self.error = true;
    }
    /**  get associated test data if any */
    pub fn get_test_data(&self, tag: &str) -> String {
        // #[cfg(test)]
        // {
        self.test_data
            .get(tag)
            .expect(&format!("no tag: {}", tag))
            .to_string()
        // }
        // #[cfg(not(test))]
        // {
        //     panic!("bad test data");
        // }
    }
    /**  set all the test data */
    pub fn set_test_data(&mut self, key: &str, val: &str) {
        // #[cfg(test)]
        // {
        self.test_data.insert(key.to_string(), val.to_string());
        // }
        // #[cfg(not(test))]
        // {
        //     panic!("bad test data");
        // }
    }
    /**  set all the test data */
    pub fn set_all_test_data(&mut self, test_data: HashMap<String, String>) {
        // #[cfg(test)]
        // {
        self.test_data = test_data;
        // }
        // #[cfg(not(test))]
        // {
        //     panic!("bad test data");
        // }
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
    /// destroy the application
    Destroy,
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
