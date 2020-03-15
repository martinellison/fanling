/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! Fanling10 is the main program for the PC version of Fanling.

## Fanling

Fanling is a distributed note-taking system that is currently implemented on:

* Linux PC
* Android

It should alse be possible to build Fanling on Microsoft Windows and
Apple PC platforms. It should also be possible to write an iPhone port
of the Android version.

Most of the functionality of Fanling is contained in the
[`fanling_interface::Engine`], which is architecture-independent and
shared between the architecture-specific main programs.

## This program (Fanling10)

This program (Fanling10) provides the code specifically for a PC.

The main functionality areas of this program are to:

* read configuration parameters from a configuration file (or the command line);
* create a web view;
* create an [`fanling_interface::Engine`] which does most of the functionality;
* connect the [`fanling_interface::Engine`] to the web view.

*/
//#![windows_subsystem = "windows"]
#[macro_use]
extern crate serde_derive;
#[cfg(not(target_os = "android"))]
extern crate web_view;
//#[macro_use]
extern crate config;
//extern crate serde_json;
extern crate structopt;
use ansi_term::Colour::*;
use ansi_term::Style;
//use log::trace;

use fanling_interface::Engine;
use std::path::PathBuf;
use std::thread;
use std::time::SystemTime;
use structopt::StructOpt;
#[cfg(not(target_os = "android"))]
use web_view::*;
#[macro_use]
extern crate quick_error;
quick_error! {
#[derive(Debug)]
/** An error found in the main program */
    pub enum Fanling10Error {
        /// engine error
        Engine(err: fanling_engine::FanlingError){from() cause(err) description(err.description())}
        /// web error
        WebView(err: web_view::Error) {from() cause(err) description(err.description())}
        /// repository-related error
        Repo(err: taipo_git_control::RepoError)  {from() cause(err) description(err.description())}
        /// generic error
        Gen(err: std::boxed::Box<dyn std::error::Error>)  {from() description(err.description())}
        /// system time error
        Time(err: std::time::SystemTimeError)  {from() cause(err) description(err.description())}
        /// error with configuration file
        Config(err: config::ConfigError) {from() cause(err) description(err.description())}
    /// error
    Internal(msg: String) {from()}
    }}
impl Fanling10Error {
    pub fn new(txt: &str) -> Self {
        Self::Internal(txt.to_string())
    }
}
type NullResult = Result<(), Fanling10Error>;
#[derive(StructOpt, Debug, Deserialize)]
#[structopt(name = "rprogress", about = "Track progress in learning")]
/// Options from the command line
pub struct Opt {
    ///Database file
    #[structopt(parse(from_os_str), short = "d", long = "db", default_value = "")]
    database_path: PathBuf,
    /// path to the git repository
    #[structopt(parse(from_os_str), short = "r", long = "repo", default_value = "")]
    repo: PathBuf,
    ///Repository branch
    #[structopt(parse(from_str), short = "b", long = "branch")]
    repo_branch: Option<String>,
    ///Repository remote
    #[structopt(parse(from_str), long = "remote")]
    repo_remote: Option<String>,
    /// name for git repo
    #[structopt(parse(from_str), short = "n", long = "name", default_value = "")]
    name: String,
    /// URL for git repo
    #[structopt(parse(from_str), short = "u", long = "url")]
    url: Option<String>,
    /// email for git repo
    #[structopt(parse(from_str), short = "e", long = "email", default_value = "")]
    email: String,
    /// verbose
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
    /// webview debug
    #[structopt(long = "debug")]
    debug: bool,
    /// prefix for identifiers
    #[structopt(parse(from_str), short = "p", long = "prefix", default_value = "?")]
    uniq_pfx: String,
    /// configuration file (will override all values)
    #[structopt(parse(from_os_str), short = "c", long = "config", default_value = "")]
    config: PathBuf,
    /// the directory within the repo containing items
    #[structopt(parse(from_str), long = "itemdir", default_value = "items")]
    item_dir: String,
    /// whether to write to the remote server    
    #[structopt(long = "nowrite")]
    no_write_to_server: bool,
    /** automatically generate items for missing items in links */
    #[structopt(long = "autolink")]
    auto_link: bool,
    /// path to the ssh credentials
    #[structopt(parse(from_os_str), short = "s", long = "ssh", default_value = "")]
    ssh_path: PathBuf,
    /// whether to slurp ssh files
    #[structopt(long = "slurp-ssh")]
    slurp_ssh: bool,
}
/** used by [web_view::WebView] */
struct UserData {
    engine: fanling_engine::FanlingEngine,
}
impl Drop for UserData {
    fn drop(&mut self) {
        trace(Blue.on(White), "dropping userdata");
    }
}

#[cfg(target_os = "android")]
/* dummy to make compile */
fn main() {}
#[cfg(not(target_os = "android"))]
fn main() {
    let mark = Mark::new();
    actual_main().expect("internal error");
    trace(Blue.on(White), "really ending main.");
    mark.touch();
}
fn actual_main() -> NullResult {
    trace(Blue.on(White), "starting main");

    let mut opt = Opt::from_args();
    let config_filename = opt
        .config
        .to_str()
        .ok_or_else(|| Fanling10Error::new("bad config file name"))?;
    if config_filename != "" {
        trace(Blue.on(White), "getting config from file");
        let mut config = config::Config::default();
        config.set_default("config", "")?;
        config.set_default("verbose", "false")?;
        config.set_default("debug", "false")?;
        config.set_default("repo_branch", None as Option<String>)?;
        config.set_default("repo_remote", None as Option<String>)?;
        config.set_default("item_dir", "items")?;
        config.set_default("no_write_to_server", "false")?;
        config.set_default("autolink", "false")?;
        config.merge(config::File::with_name(config_filename))?;
        opt = config.try_into()?;
    }
    let verbose = opt.verbose;
    if verbose {
        trace(Black.on(White), &format!("options: {:?}", opt));
    }
    // let _extra_js = " /* extra js goes here  ";
    let options = fanling_engine::EngineOptions {
        repo_options: taipo_git_control::RepoOptions {
            path: opt.repo.clone().into_boxed_path(),
            name: opt.name.clone(),
            email: opt.email.clone(),
            url: opt.url.clone(),
            required_branch: opt.repo_branch.clone(),
            required_remote: opt.repo_remote.clone(),
            item_dir: opt.item_dir.clone(),
            write_to_server: !opt.no_write_to_server,
            ssh_path: opt.ssh_path.clone().into_boxed_path(),
            slurp_ssh: opt.slurp_ssh,
        },
        interface_type: fanling_engine::InterfaceType::PC,
        search_options: fanling_engine::SearchOptions {
            database_path: opt.database_path.to_string_lossy().to_string(),
        },
        uniq_pfx: opt.uniq_pfx.clone(),
        auto_link: opt.auto_link,
    };
    //  let mut engine = fanling_engine::FanlingEngine::new(&options)?;
    trace(
        Blue.on(White),
        &format!(
            "thread is {:?}, options are {:#?}",
            thread::current().id(),
            &options
        ),
    );
    trace(Blue.on(White), "running engine with webview...");
    run_engine_with_webview(options, &opt)?;
    trace(Blue.on(White), "finished running engine with webview");
    //  engine.touch();
    Ok(())
}
fn run_engine_with_webview(
    //  engine: &mut fanling_engine::FanlingEngine,
    options: fanling_engine::EngineOptions,
    opt: &Opt,
) -> NullResult {
    trace(Blue.on(White), "building webview...");
    let verbose = opt.verbose;
    let p = UserData {
        engine: fanling_engine::FanlingEngine::new(&options)?,
    };
    {
        let now = SystemTime::now();
        let webview = web_view::builder()
            .title("Fanling 10")
            .content(Content::Html(p.engine.initial_html()?))
            .size(640, 960)
            .resizable(true)
            .debug(opt.debug)
            .user_data(p)
            .invoke_handler(invoke_handler)
            .build()?;
        trace(Blue.on(White), "webview built.");
        if verbose {
            trace(
                Black.on(White),
                &format!("web view ready, thread is {:?}", thread::current().id()),
            );
        }
        trace(
            Blue.on(White),
            &format!(
                "building webview took {}s ",
                now.elapsed()?.as_millis() as f64 / 1000.0,
            ),
        );
        // trace(Blue.on(White), "sending start.");
        // let _response = p.engine.handle_event( &fanling_interface::CycleEvent::StartPC)?;
        let mut rres = webview.run()?;
        trace(Blue.on(White), "sending end.");
        let _response = rres
            .engine
            .handle_event(&fanling_interface::CycleEvent::StopPC)?;
        trace(Blue.on(White), "run.");
    }

    trace(Blue.on(White), "run, ending main.");
    Ok(())
}
fn invoke_handler(webview: &mut WebView<UserData>, arg: &str) -> WVResult {
    trace(
        Black.on(White),
        &format!(
            "main handler, arg {}, thread is {:?}",
            arg,
            thread::current().id()
        ),
    );
    //   webview.set_title(&format!("Fanling10"))?;
    let response = webview.user_data_mut().engine.execute(arg);
    handle_response(webview, &response, arg);
    Ok(())
}
fn handle_response(
    webview: &mut WebView<UserData>,
    response: &fanling_interface::TPResult<fanling_interface::Response>,
    arg: &str,
) {
    trace(Blue.on(Yellow), "handling response...");
    match response {
        Ok(r) => {
            if r.is_shutdown_required() {
                trace(Red.on(White), "exiting");
                webview.exit();
            }
            for (t, v) in r.get_tags() {
                //FIXME: does not like new lines
                trace(Blue.on(Yellow), &format!("{} to be set to: {}", t, v));
                let vv = v.replace("\n", " ");
                let js = format!("setTag({}, {});", js_quote(&t), js_quote(&vv));
                //     trace(Green.on(Black), &format!("exec js: {}", js));
                if let Err(e) = webview.eval(&js) {
                    trace(Black.on(White), &format!("eval error {:?} for {}", e, js));
                }
            }
        }
        Err(e) => {
            trace(Red.on(White), "system error during execution");
            trace(Red.on(White), &format!("command was {:#?}", arg));
            trace(Red.on(White), &format!("error was {:#?}", e));
            let js = format!(
                "setTag({}, {});",
                js_quote("error"),
                js_quote(&format!("system error: {}", e))
            );
            //    trace(Black.on(White), &format!("exec js: {}", js));
            if let Err(e) = webview.eval(&js) {
                trace(Black.on(White), &format!("eval error {:?}", e));
            }
        }
    }
    trace(Blue.on(Yellow), "handled response.");
}
fn trace(style: Style, s: &str) {
    println!("main {}", style.paint(s));
}
fn js_quote(s: &str) -> String {
    r#"""#.to_string() + &s.replace(r#"""#, r#"\""#) + r#"""#
}
/** for debugging */
struct Mark {}
impl Mark {
    fn new() -> Mark {
        trace(Yellow.on(Cyan), "making mark in main");
        Self {}
    }
    fn touch(&self) {
        trace(Yellow.on(Cyan), "touching mark in main");
    }
}
impl Drop for Mark {
    fn drop(&mut self) {
        trace(Yellow.on(Cyan), "dropping mark in main");
    }
}
