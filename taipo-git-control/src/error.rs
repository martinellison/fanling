/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! repository errors and results */
use crate::shared::trace2;
#[macro_export]
/** create an error with a string message */
macro_rules! repo_error {
    ($msg:expr) => {
        RepoError::new(&format!(
            "repo error: {} at {}:{}:{}",
            $msg,
            file!(),
            line!(),
            column!()
        ))
    };
}
quick_error! {
   #[derive(Debug)]
    /** Error found during repository management */
    pub  enum RepoError {
        /// error found by git2
        Git(err: git2::Error) {from() cause(err)
                                                   description(err.description())}
        /// error found by git control
        Repo(text: String) {from() description(text)}
        /// internal IO error
        Io(err: std::io::Error) {from() cause(err)
                                                   description(err.description())}
        /// internal string error
        Utf8(err: std::str::Utf8Error) {from() cause(err)
                                                   description(err.description())}
        /// internal time error
        Time(err: std::time::SystemTimeError)  {from() cause(err)
                                                   description(err.description())}
        /// internal conversion error
        Convert(err: std::num::TryFromIntError)  {from() cause(err)
                                                   description(err.description())}
        /// error on conversion from UTF8 bytes
        Utf8string(err: std::string::FromUtf8Error)  {from() cause(err)
                                                      description(err.description())}
    }
}
impl RepoError {
    pub(crate) fn new(text: &str) -> RepoError {
        RepoError::Repo(text.to_owned())
    }
    pub(crate) fn dump(&self, file: &str, line: u32, col: u32) -> &Self {
        trace2(&format!(
            "repo error found: {} at {}:{}:{}",
            &format!("{:?}", &self).replace("\\n", "\n"),
            &file,
            &line,
            &col
        ));
        self
    }
}

macro_rules! dump_error {
    ($err:expr) => {
        match $err {
            Ok(x) => x,
            Err(e) => {
                let re = RepoError::from(e);
                re.dump(file!(), line!(), column!());
                if !cfg!(android) {
                    panic!("git error");
                }
                return Err(repo_error!("should not come here"));
            }
        }
    };
}
/// a result with payload
pub type RepoResult<T> = Result<T, RepoError>;
/// a result with no payload
pub type NullResult = RepoResult<()>;
