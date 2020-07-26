/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! shared code for git repositoty managemant */

//#[macro_use]
use crate::error::*;

use crate::repo::SSL_KEY_FILE;
use git2::Oid;
use log::trace;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug)]
/** data for creating a new [`FanlingRepository`] */
pub struct RepoOptions {
    /** path for repository location */
    pub path: Box<Path>,
    /** name for Git commits */
    pub name: String,
    /** email  for Git commits */
    pub email: String,
    /** URL of remote repository (if required) */
    pub url: Option<String>,
    /** remote to pull/push */
    pub required_remote: Option<String>,
    /** branch to use */
    pub required_branch: Option<String>,
    /** the directory within the repo containing items */
    pub item_dir: String,
    /** whether to write to the remote server */
    pub write_to_server: bool,
    /** ssh path */
    pub ssh_path: Box<Path>,
    /** whether to slurp ssh files */
    pub slurp_ssh: bool,
}
impl RepoOptions {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Default for RepoOptions {
    fn default() -> Self {
        Self {
            path: PathBuf::from("testfiles/test").into_boxed_path(),
            name: "".to_owned(),
            email: "".to_owned(),
            url: None,
            required_remote: Some("origin".to_owned()),
            required_branch: Some("main".to_owned()),
            item_dir: "items".to_owned(),
            write_to_server: false,
            ssh_path: PathBuf::from(SSL_KEY_FILE).into_boxed_path(), /* ?? */
            slurp_ssh: false,
        }
    }
}

/** An identifier understood to refer to a blob. */
#[derive(Default, Clone, Eq, PartialEq)]
pub struct RepoOid {
    bytes: [u8; 20],
}
impl RepoOid {
    /*x pub */
    pub(crate) fn to_oid(&self) -> Result<Oid, RepoError> {
        Ok(dump_error!(Oid::from_bytes(&self.bytes)))
    }
    /** convert a Git2 [Oid] to a RepoOid (a byte array) */
    pub(crate) fn from_oid(oid: &Oid) -> Self {
        let mut ro: Self = Default::default();
        ro.bytes.clone_from_slice(oid.as_bytes());
        ro
    }
    /** suitable initial value */
    pub fn new() -> Self {
        Self { bytes: [0u8; 20] }
    }
}
impl fmt::Debug for RepoOid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //    write!(f, "{:x}", &self.bytes.as_hex())
        write!(f, "{:?}", self.to_oid().expect("bad???"))
    }
}
#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum StructureStatus {
    Good,
    BadHead,
    HeadNotBranch,
    NoSubTree,
}

//#[derive(Debug)]
#[derive(Clone)]
/** kind of operation to apply to an object */
pub enum ObjectOperation {
    Add(String),
    Modify(String),
    Delete,
    Conflict {
        base: Option<RepoOid>,
        ours: Option<RepoOid>,
        theirs: Option<RepoOid>,
    },
    Fix(String),
    Unknown,
}

impl std::fmt::Debug for ObjectOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectOperation::Add(oid) => write!(f, "add {:?}", oid),
            ObjectOperation::Modify(oid) => write!(f, "modify {:?}", oid),
            ObjectOperation::Delete => write!(f, "delete"),
            ObjectOperation::Conflict {
                base: _base_oid,
                ours: ours_oid,
                theirs: _theirs_oid,
            } => write!(f, "conflict ours: {:?}", ours_oid),
            ObjectOperation::Fix(oid) => write!(f, "fix {:?}", oid),
            ObjectOperation::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone)]
/** a change to an object */
pub struct Change {
    /// operation to be applied to an item
    pub op: ObjectOperation,
    ///the path within the repository of the item
    pub path: String,
    /// description for commit message
    pub descr: String,
}
impl Change {
    pub fn new(op: ObjectOperation, path: String, descr: String) -> Self {
        Self { op, path, descr }
    }
    pub fn with_oid(self, oid: RepoOid) -> ChangeWithOid {
        ChangeWithOid { change: self, oid }
    }
}
// impl<T> Clone for Change {
//     fn clone(&self) -> Self {
//         Self {op, path, }
//     }
// }
pub struct ChangeWithOid {
    // a change
    pub change: Change,
    // the repo oid for the string (if any)
    pub oid: RepoOid,
}
/** a set of [`Change`]s */
pub type ChangeList = Vec<Change>;
/** a set of [`ChangeWithOid`]s */
pub type ChangeWithOidList = Vec<ChangeWithOid>;

/** data about an item as retrieved from git */
#[derive(Debug)]
pub struct EntryDescr {
    /** the path within the repository to the entry */
    pub path: String,
    pub oid: RepoOid,
    pub kind: String,
    pub blob: String,
}
/** time an operation and output start and end messages */
pub struct Timer {
    start: SystemTime,
    descr: String,
}
impl Timer {
    pub fn new(descr: &str, loc: &str) -> Self {
        trace2(&format!("starting {} at {}", descr, loc));
        Self {
            start: SystemTime::now(),
            descr: descr.to_owned(),
        }
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        trace2(&format!(
            "{} took {}s.",
            self.descr,
            self.start.elapsed().expect("bad???").as_millis() as f64 / 1000.0
        ));
    }
}
#[macro_export]
macro_rules! repo_timer {
    ($err:expr) => {
        let _timer = Timer::new($err, concat!(file!(), ":", line!()));
    };
}
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
macro_rules! repo_trace {
    ($err:expr) => {
        let _tracer = Tracer::new($err, concat!(file!(), ":", line!()));
    };
}

/** convenience function for debug traces */
pub(crate) fn trace(txt: &str) {
    trace!("{}", txt);
    println!(
        "git {}",
        ansi_term::Colour::Fixed(9)
            .on(ansi_term::Colour::Black)
            .paint(txt)
    );
}
/** convenience function for debug traces (more visibility) */
pub(crate) fn trace2(txt: &str) {
    trace!("{}", txt);
    println!(
        "git {}",
        ansi_term::Colour::Fixed(99)
            .on(ansi_term::Colour::White)
            .paint(txt)
    );
}
