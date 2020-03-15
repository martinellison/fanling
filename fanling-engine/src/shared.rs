/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! shared things that are used in more than one module */
use difference::{Changeset, Difference};
use log::trace;

/** merge two strings into one */
pub fn merge_strings(sa: &str, sb: &str) -> String {
    let changes = Changeset::new(sa, sb, " ");
    trace(&format!(
        "merge text '{}' and '{}' changes {}",
        sa, sb, changes
    ));
    let parts: Vec<String> = changes
        .diffs
        .iter()
        .map(|c| match c {
            Difference::Same(s) => s.clone(),
            Difference::Add(s) => s.clone(),
            Difference::Rem(s) => s.clone(),
        })
        .collect();
    parts.join("")
}

#[macro_export]
macro_rules! fanling_error {
    ($msg:expr) => {
        FanlingError::new(&format!(
            "error {} at {}:{}:{}",
            $msg,
            file!(),
            line!(),
            column!()
        ))
    };
}

#[macro_export]
macro_rules! dump_fanling_error {
    ($err:expr) => {
        match $err {
            Ok(x) => x,
            Err(e) => {
                let re = FanlingError::from(e);
                re.dump(file!(), line!(), column!());
                if !cfg!(android) {
                    panic!("fanling error");
                }
                return Err(fanling_error!("bad"));
            }
        }
    };
}
// #[cfg(not(target_os = "android"))]
// extern crate web_view;
quick_error! {
#[derive(Debug)]
/** Error found in Fanling */
pub enum FanlingError {
    /// repository-related error
    RepoError(err: taipo_git_control::RepoError)  {from() cause(err)   description(err.description())}
    /// error in engine
    FanlingError(msg: String) {from()}
    /// internal IO error
    Io(err: std::io::Error) {from() cause(err)   description(err.description())}
    Diesel(err: diesel::result::Error) {from() cause(err)    description(err.description())}
    Template(err: crate::askama::Error)  {from() cause(err)  description(err.description())}
    Time(err: std::time::SystemTimeError)  {from() cause(err)   description(err.description())}
    Yaml (err: serde_yaml::Error) {from() cause(err)    description(err.description())}
    DieselMigration(err:  diesel_migrations::RunMigrationsError) {from() cause(err) description(err.description())}
    Gen(err: std::boxed::Box<dyn std::error::Error>)  {from()description(err.description())}
    DieselConnection(err: diesel::ConnectionError) {from() cause(err) description(err.description())}
    Utf8Error(err:std::str::Utf8Error)  {from() cause(err) description(err.description())}

    RegexError(err:regex::Error) {from() cause(err) description(err.description())}

    TryInto(err:std::num::TryFromIntError){from() cause(err) description(err.description())}
    ParseInt(err: std::num::ParseIntError){from() cause(err) description(err.description())}
    DateTime(err: chrono::format::ParseError) {from() cause(err) description(err.description())}
    Var (err: std::env::VarError) {from() cause(err) description(err.description())}
    Serde(err: serde_json::error::Error) {from() cause(err) description(err.description())}
} }
impl FanlingError {
    pub fn new(txt: &str) -> FanlingError {
        FanlingError::FanlingError(txt.to_string())
    }
    pub(crate) fn dump(&self, file: &str, line: u32, col: u32) -> &Self {
        trace(&format!(
            "fanling error found: {} at {}:{}:{}",
            &format!("{:?}", &self).replace("\\n", "\n"),
            &file,
            &line,
            &col
        ));
        self
    }
}
/// [Result] type for this package
pub type FLResult<T> = std::result::Result<T, FanlingError>;

pub type NullResult = Result<(), FanlingError>;

/** output start and end messages */
pub struct Tracer {
    descr: String,
}
impl Tracer {
    pub fn new(descr: &str, loc: &str) -> Self {
        trace(&format!("starting {} at {}", descr, loc));
        Self {
            descr: descr.to_owned(),
        }
    }
}
impl Drop for Tracer {
    fn drop(&mut self) {
        trace(&format!("{} done.", self.descr));
    }
}
#[macro_export]
macro_rules! fanling_trace {
    ($err:expr) => {
        let _tracer = Tracer::new($err, concat!(file!(), ":", line!()));
    };
}
/** convenience function for debug traces */
pub(crate) fn trace(txt: &str) {
    trace!("{}", txt);
    println!(
        "engine {}",
        ansi_term::Colour::Purple
            .on(ansi_term::Colour::Fixed(250))
            .paint(txt)
    );
}
// macro_rules! dump_error {
//     ($err:expr) => {
//         match $err {
//             Ok(x) => x,
//             Err(e) => {
//                 let re = FanlingError::from(e);
//                 re.dump(file!(), line!(), column!());
//                 if !cfg!(android) {
//                     panic!("git error");
//                 }
//                 return Err(fanling_error!("bad"));
//             }
//         }
//     };
// }

// #[derive(RustEmbed)]
// #[folder = "embed/"]
// struct Asset;
// /** retrieve a shared string asset */
// pub fn embedded_asset(name: &str) -> FLResult<String> {
//     let asset_res = Asset::get(name);
//     if asset_res.is_none() {
//         return Err(fanling_error!(&format!(
//             "could not get asset: {}",
//             &name
//         )));
//     }
//     let asset_bytes = asset_res.expect("bad???");
//     let asset: String = std::str::from_utf8(asset_bytes.as_ref())?.to_string();
//     Ok(asset.as_str().to_string())
// }
// /** for debugging when data is dropped */
// pub struct Tracked<'a, T> {
//     tag: &'a str,
//     data: T,
// }
// impl<'a, T> Tracked<'a, T> {
//     pub fn new(tag: &'a str, data: T) -> Self {
//         trace(&format!("creating {}", tag));
//         Self { tag, data }
//     }
//     pub fn as_ref(&self) -> &T {
//         &self.data
//     }
//     pub fn as_mut(&mut self) -> &mut T {
//         &mut self.data
//     }
// }

// impl<'a, T> Drop for Tracked<'a, T> {
//     fn drop(&mut self) {
//         trace(&format!("dropping {}", self.tag));
//     }
// }
// /** convenience function for debug traces */
// fn trace(txt: &str) {
//     println!(
//         "search {}",
//         ansi_term::Colour::Yellow
//             .on(ansi_term::Colour::Green)
//             .paint(txt)
//     );
// }
