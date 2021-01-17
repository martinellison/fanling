/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! repository support */
use crate::error::{NullResult, RepoError, RepoResult};
//#[macro_use]
use crate::shared::{
    trace, ChangeList, ChangeWithOid, ChangeWithOidList, EntryDescr, ObjectOperation, RepoOid,
    RepoOptions, StructureStatus, Timer, Tracer,
};
use crate::{repo_timer, repo_trace};
use git2::{build::RepoBuilder, *};
use git2_credentials::CredentialHandler;
use std::path::{Path, PathBuf};
/** name of the SSL key file */
pub const SSL_KEY_FILE: &str = "id_rsa";
use std::convert::TryInto;
use std::fmt;
use std::str;
//use std::time::Duration;
use std::fs;
use std::thread;

//use std::os::unix::fs::PermissionsExt;

use std::time::SystemTime;

/** whether any further action is required on a newly-opened repo */
#[derive(Debug)]
pub enum RepoActionRequired {
    NoAction,
    LoadAll,
    ProcessChanges,
}

/** A git repository under management */
pub struct FanlingRepository {
    /** path for repository location */
    pub path: Box<Path>,
    /** the Git repository */
    repo: Repository,
    /** the user for signing commits */
    signature: Signature<'static>,
    /** does the repo have unpushed changes */
    needs_push: bool,
    /** URL of remote repository (if required) */
    url: Option<String>,
    /** remote of the remote repo  */
    required_remote: String,
    /** the branch to use */
    required_branch: String,
    /** the directory within the repo containing items */
    item_dir: String,
    /** whether to write to the remote server */
    write_to_server: bool,
    /** ssh path */
    ssh_path: Box<Path>,
    /** whether to slurp ssh files */
    slurp_ssh: bool,
}
impl FanlingRepository {
    /*  Creating repository */
    /** ensure that the repo is open (creating it or cloning it if necessary) */
    pub fn new_open(opts: &RepoOptions) -> RepoResult<(FanlingRepository, RepoActionRequired)> {
        repo_trace!("repo new open");
        trace(&format!("options {:?}", opts));
        let (mut repo, repo_action_required) = Self::new_from_options(opts)?;
        trace("repo ok, checking structure");
        let struct_status = repo.check_structure()?;
        if struct_status == StructureStatus::BadHead {
            repo.create_initial()?;
        }
        if struct_status != StructureStatus::Good {
            trace(&format!("structure status is {:?}", struct_status));
            let tree_oid = repo.get_latest_tree()?.id();
            repo.create_structure(tree_oid)?;
        };
        Ok((repo, repo_action_required))
    }
    /** create a repository object (creating it or cloning it if necessary) */
    fn new_from_options(opts: &RepoOptions) -> RepoResult<(FanlingRepository, RepoActionRequired)> {
        repo_trace!("new from options");
        if Self::can_open_repo(&opts.path)? {
            trace(&format!("need to open existing repo {:?}", opts.path));
            Ok((Self::open(opts)?, RepoActionRequired::ProcessChanges))
        } else if opts.url.is_none() {
            trace("no repo, no remote, need to init repo");
            Ok((Self::init(opts)?, RepoActionRequired::NoAction))
        } else {
            trace("no repo, remote, need to clone");
            Ok((Self::clone_repo(opts)?, RepoActionRequired::LoadAll))
        }
    }
    /** initialise a new repository */
    fn init(opts: &RepoOptions) -> RepoResult<FanlingRepository> {
        let path = opts.path.clone();
        repo_timer!(&format!("init repo {:?}", path));
        fs::create_dir_all(path.clone())?;
        let r = dump_error!(Repository::init_bare(path));
        Ok(Self::new(r, opts)?)
    }
    /** `can_open_repo` checks whether there is a repository that can be opened */
    fn can_open_repo(repo_path: &Path) -> RepoResult<bool> {
        if !repo_path.exists() {
            trace("repo does not exist");
            return Ok(false);
        }
        if !repo_path.is_dir() {
            trace("repo is not dir");
            return Ok(false);
        }
        let mut has_entry = false;
        for entry in repo_path.read_dir()? {
            if let Err(_e) = entry {
                trace("has bad entry");
                return Ok(false);
            } else {
                has_entry = true;
            }
        }
        if !has_entry {
            trace("repo has no files");
            return Ok(false);
        }
        Ok(true)
    }
    /** open an existing repository */
    fn open(opts: &RepoOptions) -> RepoResult<FanlingRepository> {
        repo_timer!("open repo");
        trace(&format!("opening repo: {:?}", &opts.path));
        if opts.path.exists() {
            if opts.path.is_dir() {
                trace("directory found, listing:");
                for entry in opts.path.read_dir()? {
                    match entry {
                        Ok(_ent) => {
                            //  trace(&format!("dir has {:?}", ent.path()));
                        }
                        Err(e) => {
                            trace(&format!("bad entry {:?}", e));
                        }
                    }
                }
            } else {
                trace(&format!("{:?} is not a directory", &opts.path));
            }
        } else {
            // trace(&format!("path to repo : {:?}", &opts.path));
            for ap in opts.path.ancestors() {
                trace(&format!(
                    "{:?} {}",
                    ap,
                    if ap.exists() {
                        "exists"
                    } else {
                        "does not exist"
                    }
                ));
            }
        }
        let r = dump_error!(Repository::open(opts.path.clone()));
        trace("repo opened");
        Ok(Self::new(r, opts)?)
    }
    /** clone a repository from a server (a Git clone, not a Rust clone) */
    pub fn clone_repo(opts: &RepoOptions) -> RepoResult<FanlingRepository> {
        repo_timer!("clone repo");
        let mut fetch_options = FetchOptions::new();
        let mut cb = git2::RemoteCallbacks::new();
        let path = opts.path.clone();
        Self::set_remote_callbacks(
            &opts
                .ssh_path
                .to_str()
                .ok_or_else(|| repo_error!("bad path"))?
                .to_owned(),
            opts.slurp_ssh,
            &mut cb,
        )?;
        fetch_options.remote_callbacks(cb);
        let mut builder = RepoBuilder::new();
        builder.bare(false);
        builder.fetch_options(fetch_options);
        let url = opts
            .url
            .clone()
            .ok_or_else(|| repo_error!("URL must be specified for clone"))?;
        trace(&format!("dir for clone is {:?}", &path));
        // fs::create_dir_all(opts.base_path.clone())?;
        // // TODO: revisit and probably remove permission hacking
        // // get a ?permissions error in Android, maybe file does not exist or wrong file path
        // trace("ensured directory, setting permissions,,,");
        // Self::dump_file_meta(&opts.base_path.as_ref())?;
        // let permissions = Permissions::from_mode(0o744);
        // fs::set_permissions(opts.base_path.clone(), permissions)?;
        // trace("permissions set");
        // Self::dump_file_meta(&opts.base_path.as_ref())?;
        trace(&format!(
            "actually cloning (url {:?}) to {:?}...",
            &url, &path
        ));
        let r = dump_error!(builder.clone(&url, &path));
        trace("cloned.");
        Ok(Self::new(r, opts)?)
    }
    /** create a new repo object */
    fn new(repo: Repository, opts: &RepoOptions) -> RepoResult<FanlingRepository> {
        Ok(Self {
            repo,
            signature: Signature::now(&opts.name, &opts.email)?,
            needs_push: false,
            url: opts.url.clone(),
            required_remote: opts
                .required_remote
                .as_ref()
                .unwrap_or(&"origin".to_owned())
                .to_string(),
            required_branch: opts
                .required_branch
                .as_ref()
                .unwrap_or(&"main".to_owned())
                .to_string(),
            item_dir: opts.item_dir.clone(),
            path: opts.path.clone(),
            write_to_server: opts.write_to_server,
            ssh_path: opts.ssh_path.clone(),
            slurp_ssh: opts.slurp_ssh,
        })
    }
    /* ### Branches and commits */
    // /** check if branch exists/get branch */
    // fn try_get_branch(&self) -> Option<Branch> {
    //     match self
    //         .repo
    //         .find_branch(&self.required_branch, BranchType::Local)
    //     {
    //         Ok(b) => Some(b),
    //         Err(e) => {
    //             trace(&format!(
    //                 "branch {} not found {:?}",
    //                 self.required_branch, e
    //             ));
    //             None
    //         }
    //     }
    // }
    /** the state of the repository, for debugging */
    pub fn state(&self) -> String {
        format!("{:?}", self.repo.state())
    }
    // /** check if current branch is the required branch */
    // fn is_branch_correct(&self) -> RepoResult<bool> {
    //     match self.try_get_branch() {
    //         Some(b) => match b.name() {
    //             Err(_) => Ok(false),
    //             Ok(n) => match n {
    //                 Some(nnn) => Ok(nnn == self.required_branch),
    //                 None => Ok(false),
    //             },
    //         },
    //         None => Err(repo_error!("no branch")),
    //     }
    // }
    /** get last commit for branch */
    fn find_last_commit(&self) -> Result<Option<Commit<'_>>, RepoError> {
        let head = dump_error!(self.repo.head());
        let obj = dump_error!(head.resolve()?.peel(ObjectType::Commit));
        trace(&format!("last commit at {}", obj.id()));
        Ok(Some(obj.into_commit().map_err(|_e| {
            repo_error!("find last commit error: not a commit")
        })?))
    }
    // /** create new branch using a commit */
    // fn create_branch_from_commit(&self, target_commit: Commit) -> RepoResult<Branch> {
    //     Ok(self
    //         .repo
    //         .branch(&self.required_branch, &target_commit, false)?)
    // }
    // /** branch is correct */
    // fn branch_is_correct(&self) -> RepoResult<bool> {
    //     let head = self.repo.head()?;
    //     if !head.is_branch() {
    //         return Err(repo_error!("head is not branch"));
    //     }
    //     if let Some(n) = head.name() {
    //         Ok(n == self.required_branch)
    //     } else {
    //         Err(repo_error!("head has no name"))
    //     }
    // }
    /** switch to correct branch */
    fn switch_to_branch(&self) -> NullResult {
        repo_trace!("switching to branch");
        for r in self.repo.references().expect("bad reference").names() {
            trace(&format!(
                "branch could be: {}",
                r.expect("bad reference name")
            ));
        }
        Ok(dump_error!(self.repo.set_head(&self.refname())))
    }
    /** name of the required branch for git */
    fn refname(&self) -> String {
        format!("refs/heads/{}", self.required_branch)
    }
    // /** commit changes with tree builder */
    // fn commit_treebuilder(
    //     &mut self,
    //     treebuilder: &TreeBuilder,
    //     old_commit: Option<&Commit>,
    // ) -> RepoResult<Oid> {
    //     let tree_oid = dump_error!(treebuilder.write());
    //     let tree = dump_error!(self.repo.find_tree(tree_oid));
    //     //     self.commit_tree(tree, old_commit)
    //     // }
    //     // /** commit changes with tree */
    //     // fn commit_tree(&mut self, tree: Tree, old_commit: Option<&Commit>) -> RepoResult<Oid> {
    //     match old_commit {
    //         Some(c) => self.write_commit(tree, "extra commit", &[&c]),
    //         None => self.write_commit(tree, "initial commit", &[]),
    //     }
    // }
    /** commit changes */
    fn write_commit(
        &self,
        new_tree: Tree<'_>,
        message: &str,
        parent_commits: &[&Commit<'_>],
    ) -> RepoResult<Oid> {
        repo_trace!("writing commit");
        trace(&format!(
            "writing commit, new tree has {} parents, {} entries ({})...",
            parent_commits.len(),
            new_tree.len(),
            message
        ));
        //  Self::describe_tree(&new_tree, "for commit");
        for c in parent_commits {
            trace(&format!("commit parent {}", c.id()));
        }
        let update_ref = if parent_commits.len() > 0 {
            Some("HEAD")
        } else {
            None
        };
        let commit_oid = dump_error!(self.repo.commit(
            update_ref,      //  point HEAD to our new commit
            &self.signature, // author
            &self.signature, // committer
            message,         // commit message
            &new_tree,       // tree
            parent_commits,  // parents
        ));
        Ok(commit_oid)
    }
    /** latest local commit for fetch */
    pub fn our_commit(&self) -> RepoResult<Commit<'_>> {
        Ok(self
            .find_last_commit()?
            .ok_or_else(|| (repo_error!("no commit")))?)
    }
    /** latest commit on other branch after fetch */
    pub fn their_commit(&self) -> RepoResult<Commit<'_>> {
        let their_reference = self.repo.find_reference("FETCH_HEAD")?;
        Ok(their_reference.peel_to_commit()?)
    }

    /* ### Trees and tree entries */
    /** check structure

    assumes head */
    fn check_structure(&self) -> RepoResult<StructureStatus> {
        repo_trace!("check structure");
        // error in Android is somewhere after here but before any other trace statements
        let head = match self.repo.head() {
            Ok(h) => {
                repo_trace!("found head");
                h
            }
            Err(e) => {
                trace(&format!("error from head: {:?}", e));
                repo_trace!("structure is: bad head");
                return Ok(StructureStatus::BadHead);
            }
        };
        repo_trace!("have head");
        if !head.is_branch() {
            repo_trace!("structure is: head not branch");
            return Ok(StructureStatus::HeadNotBranch);
        }
        let tree = dump_error!(head.peel_to_tree());
        repo_trace!("have tree");
        let sub_tree = self.try_get_subtree(tree)?;
        repo_trace!("have subtree");
        if sub_tree.is_none() {
            repo_trace!("structure is: no subtree");
            return Ok(StructureStatus::NoSubTree);
        }
        repo_trace!("structure is good");
        Ok(StructureStatus::Good)
    }
    /** create structure */
    fn create_structure(
        &mut self,
        old_top_tree_oid: Oid,
        //  old_commit: Option<&Commit>,
    ) -> RepoResult<Oid> {
        repo_trace!("create structure");
        let sub_tree_oid = {
            repo_trace!("create sub tree");
            let sub_tree_builder = self.treebuilder_with_readme(None, "Items go here", "a")?;
            dump_error!(sub_tree_builder.write())
        };
        let sub_dir_name = &self.item_dir.clone();
        // let mut old_top_tree_oid = None;
        // if let Some(oid) = old_top_tree_oid {
        //     old_top_tree_oid = Some(self.repo.find_tree(oid)?.id());
        // }
        //  let mut old_top_tree_oid  = match
        let tree_oid = {
            repo_trace!("create parent tree");
            let mut parent_tree_builder =
                self.treebuilder_with_readme(Some(old_top_tree_oid), "Fanling repo", "b")?;
            Self::insert_directory(&mut parent_tree_builder, sub_dir_name, sub_tree_oid)?;
            dump_error!(parent_tree_builder.write())
        };
        let commit_oid = self.commit_tree(tree_oid)?;
        Ok(commit_oid)
    }
    /** create an initial commit */
    fn create_initial(&mut self) -> NullResult {
        let tree_oid = {
            let treebuilder = self.treebuilder_with_readme(None, "initial commit", "c")?;
            dump_error!(treebuilder.write())
        };
        let tree = dump_error!(self.repo.find_tree(tree_oid));
        //trace(&format!("tree has {} entries", tree.len()));
        Self::describe_tree(&tree, "create_initial:initial");
        let oid = self.write_commit(tree, "Initial commit", &[])?;
        let branch = dump_error!(self.repo.find_commit(oid));
        dump_error!(self.repo.branch(&self.required_branch, &branch, false));
        self.switch_to_branch()?;
        Ok(())
        //  self.commit_treebuilder(&tb, None)
    }
    /** commit a tree */
    fn commit_tree(&mut self, tree_oid: Oid) -> RepoResult<Oid> {
        let parent_commit = self
            .find_last_commit()?
            .ok_or_else(|| (repo_error!("no commit")))?;
        let tree = dump_error!(self.repo.find_tree(tree_oid));
        // trace(&format!("tree has {} entries", tree.len()));
        Self::describe_tree(&tree, "commit_tree:commit");
        self.write_commit(tree, "Initial commit", &[&parent_commit])
    }
    /** creates a treebuilder with a readme file (so git will create
    the directory) . Specify `tree_oid` to use an existing tree. */
    fn treebuilder_with_readme(
        &mut self,
        tree_oid: Option<Oid>,
        readme_text: &str,
        test_mark: &str, /* for debugging */
    ) -> RepoResult<TreeBuilder<'_>> {
        repo_trace!("building treebuilder with readme");
        let mut new_tree_builder = match tree_oid {
            None => {
                trace("making new tree");
                self.repo.treebuilder(None)?
            }
            Some(oid) => {
                trace(&format!("have oid {}, finding tree...", &oid));
                let tree = self.repo.find_tree(oid)?;
                //  trace("making tree builder...");
                self.repo.treebuilder(Some(&tree))?
            }
        };
        trace("inserting file...");
        //let oid = self.repo.blob(readme_text.to_string().as_bytes())?;
        let path = format!("README-{}.md", test_mark);
        self.insert_file(&mut new_tree_builder, readme_text, &path)?;
        Ok(new_tree_builder)
    }
    /** insert directory item into tree builder

    set the entry for the item subdirectory in the parent tree builder */
    fn insert_directory(
        parent_tree_builder: &mut TreeBuilder<'_>,
        sub_dir_name: &str,
        subtree_oid: Oid,
    ) -> NullResult {
        trace(&format!("inserting subtree, dir name {}...", &sub_dir_name));
        dump_error!(parent_tree_builder.insert(
            sub_dir_name,
            subtree_oid,
            0o0040000, /* directory */
        ));
        trace("inserted subtree.");
        Ok(())
    }

    /** insert blob (file) into tree */
    fn insert_file(
        &self,
        parent_tree_builder: &mut TreeBuilder<'_>,
        text: &str,
        path: &str,
    ) -> NullResult {
        trace(&format!("inserting file, path {}", &path));
        let oid = self.repo.blob(text.to_string().as_bytes())?;
        self.insert_blob(parent_tree_builder, oid, path)
    }
    /** insert blob (file) into tree */
    fn insert_blob(
        &self,
        parent_tree_builder: &mut TreeBuilder<'_>,
        oid: Oid,
        path: &str,
    ) -> NullResult {
        trace(&format!("inserting blob {} at {}", &path, &oid));
        dump_error!(parent_tree_builder.insert(path, oid, 0o100644 /* regular */));
        trace("inserted");
        Ok(())
    }
    /** check/get directory entry from tree */
    fn try_get_subtree(&self, parent_tree: Tree<'_>) -> RepoResult<Option<Tree<'_>>> {
        //  trace(&format!("parent tree has {} entries", parent_tree.len()));
        Self::describe_tree(&parent_tree, "try_get_subtree:parent");
        match parent_tree.get_name(&self.item_dir) {
            Some(n) => {
                let obj = dump_error!(n.to_object(&self.repo));
                let tree = dump_error!(obj.peel_to_tree());
                //  trace(&format!("subtree has {} entries", tree.len()));
                Self::describe_tree(&tree, "try_get_subtree:subtree");
                Ok(Some(tree))
            }
            None => Ok(None),
        }
    }
    /** check if repo contains file by path */
    pub fn repo_has_file(&self, path: &str) -> RepoResult<bool> {
        let tree = self.get_latest_tree()?;
        //  trace(&format!("latest tree has {} entries", tree.len()));
        Self::describe_tree(&tree, "epo_has_file:latest");
        let subtree = self
            .try_get_subtree(tree)?
            .ok_or_else(|| repo_error!("no subtree"))?;
        //   trace(&format!("subtree has {} entries", subtree.len()));
        Self::describe_tree(&subtree, "epo_has_file:subtree");
        Ok(self.has_file(subtree, path))
    }

    /** check if tree contains file by path */
    fn has_file(&self, tree: Tree<'_>, path: &str) -> bool {
        match tree.get_name(path) {
            None => false,
            Some(n) => n.kind() == Some(ObjectType::Blob),
        }
    }
    /** get the latest top-level tree in the repo (on the required branch) */
    fn get_latest_tree(&self) -> Result<Tree<'_>, RepoError> {
        repo_trace!("get latest tree");
        let reference = dump_error!(self.repo.head());
        let tree = dump_error!(reference.peel_to_tree());
        //  trace(&format!("latest tree has {} entries", tree.len()));
        Self::describe_tree(&tree, "get_latest_tree:latest");
        Ok(tree)
    }
    /** check/get blob (file) entry from tree */
    /* ### Other in repo */
    /** changes since specified commit */
    /* ### Remotes and remote (server) repositories */
    /** does this repo have a remote? */
    pub fn has_remote(&self) -> bool {
        self.url.is_some()
    }
    /** check if have/get remote */
    fn try_get_remote(&self) -> RepoResult<Remote<'_>> {
        let url = match self.url.clone() {
            Some(u) => u,
            None => {
                return Err(repo_error!("no url"));
            }
        };
        match self.repo.find_remote(&self.required_remote) {
            Ok(r) => Ok(r),
            Err(_) => Ok(dump_error!(self.repo.remote(&self.required_remote, &url))),
        }
    }
    /** set that the repo needs to be pushed (only if there us a remote to push to) */
    pub fn set_needs_push(&mut self) {
        if self.url.is_some() && self.write_to_server {
            trace("setting needs push");
            self.needs_push = true;
        } else {
            // debug
            trace(&format!(
                "not setting push because: has url {:?}, writing to server {:?}",
                self.url.is_some(),
                self.write_to_server
            ));
        }
    }
    /** whether repo needs to push */
    pub fn does_need_pushing(&self) -> bool {
        self.needs_push
    }
    /** fetch from server */
    pub fn fetch(&mut self) -> NullResult {
        repo_timer!("fetch repo");
        assert!(self.url.is_some(), "fetching but no remote repo");
        let mut fetch_options = FetchOptions::new();
        let mut cb = git2::RemoteCallbacks::new();
        Self::set_remote_callbacks(
            &self
                .ssh_path
                .to_str()
                .ok_or_else(|| repo_error!("bad path"))?
                .to_owned(),
            self.slurp_ssh,
            &mut cb,
        )?;
        fetch_options.remote_callbacks(cb);
        trace(&format!("finding remote: {}...", &self.required_remote));
        let mut remote = dump_error!(self.repo.find_remote(&self.required_remote));
        trace(&format!("fetching (branch: {})...", &self.required_branch));
        dump_error!(remote.fetch(&[&self.required_branch], Some(&mut fetch_options), None));
        trace("fetched.");
        Ok(())
    }
    /** merge the versions and determine the status (no change/fast forward/conflict) */
    pub fn merge(&mut self) -> RepoResult<MergeOutcome> {
        repo_trace!("merge");
        let our_commit = self.our_commit()?;
        let their_commit = self.their_commit()?;
        trace(&format!(
            "commits ours {} theirs {}",
            our_commit.id(),
            their_commit.id()
        ));
        if our_commit.id() == their_commit.id() {
            trace("commits are the same, not merging");
            return Ok(MergeOutcome::AlreadyUpToDate);
        }
        // FUTURE: maybe use merge analysis
        trace("commits different, so merging and finding conflicts...");
        let index = dump_error!(self.repo.merge_commits(
            &our_commit,
            &their_commit,
            Some(&MergeOptions::new())
        ));
        if index.has_conflicts() {
            trace("merge conflicts exist");
            self.dump_conflicts(&index)?;
            return Ok(MergeOutcome::Conflict(index));
        }
        Ok(MergeOutcome::Merged(index))
    }
    // /** index of merge after fetch */
    // pub fn merge_index(&self, our_commit: Oid, their_commit: Oid) -> NullResult {
    //     let ours = self.repo.find_commit(our_commit)?;
    //     let theirs = self.repo.find_commit(their_commit)?;
    //     self.repo
    //         .merge_commits(&ours, &theirs, Some(&MergeOptions::new()))?;
    //     Ok(())
    // }
    /** apply a change list to an index (to resolve conflicts)

    See
    [index format](https://github.com/git/git/blob/master/Documentation/technical/index-format.txt)
    for the details of the git index entry format.*/
    pub fn apply_changelist_to_index(&self, changes: &ChangeList, index: &mut Index) -> NullResult {
        for change in changes {
            trace(&format!("applying operation {:?}", change.op));
            let now = dump_error!(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH));
            let index_time_now = IndexTime::new(now.as_secs().try_into()?, now.subsec_nanos());
            match &change.op {
                ObjectOperation::Add(data) | ObjectOperation::Modify(data) => {
                    //                    let data = self.repo.find_blob(o.to_oid()?)?;
                    let entry = IndexEntry {
                        ctime: index_time_now,
                        mtime: index_time_now,
                        dev: 0,
                        ino: 0,
                        mode: 0o100644, // regular file
                        uid: 0,
                        gid: 0,
                        file_size: 0,
                        id: Oid::zero(),
                        flags: 0,
                        flags_extended: 0,
                        path: change.path.clone().into_bytes(), //?? does path include "items/"?
                    };
                    //   trace(&format!("entry ", &entry));
                    dump_error!(index.add_frombuffer(&entry, data.as_bytes()));
                }
                ObjectOperation::Delete => {
                    dump_error!(index.remove_path(&PathBuf::from(change.path.clone())));
                }
                _ => return Err(repo_error!("invalid operation")),
            }
        }
        Ok(())
    }
    /** apply a change list to a merge outcome (to resolve conflicts) */
    pub fn apply_changes_to_merge_outcome(
        &self,
        changes: &ChangeList,
        mo: &mut MergeOutcome,
    ) -> NullResult {
        if let MergeOutcome::Conflict(index) = mo {
            trace(&format!(
                "merge outcome had conflict, applying {} changes to index",
                changes.len()
            ));
            self.repo.set_index(index)?;
            self.apply_changelist_to_index(changes, index)
        } else {
            Err(repo_error!("should not come here: should be conflict"))
        }
    }
    /** Apply commit after merge */
    pub fn commit_merge(
        &self,
        mo: &mut MergeOutcome, // new_tree_oid: RepoOid,
                               // message: &str,
                               // parent_commits: &[&Commit]
    ) -> RepoResult<Option<Oid>> {
        match mo {
            MergeOutcome::AlreadyUpToDate => {
                repo_trace!("merge outcome already up to date, no commit required");
                Ok(None)
            }
            MergeOutcome::Merged(ix) => {
                repo_trace!("merge outcome merged, commit required");
                //   Ok(None)
                let our_commit = self.our_commit()?;
                let their_commit = self.their_commit()?;
                let new_tree_oid = dump_error!(ix.write_tree_to(&self.repo));
                let new_tree = dump_error!(self.repo.find_tree(new_tree_oid));
                //  trace(&format!("new tree has {} entries", new_tree.len()));
                Self::describe_tree(&new_tree, "commit_merge: merged");
                let message = "merge after fetch (merged)";
                Ok(Some(self.write_commit(
                    new_tree,
                    message,
                    &[&our_commit, &their_commit],
                )?))
            }
            MergeOutcome::Conflict(ix) => {
                let our_commit = self.our_commit()?;
                let their_commit = self.their_commit()?;
                let new_tree_oid = dump_error!(ix.write_tree_to(&self.repo));
                let new_tree = dump_error!(self.repo.find_tree(new_tree_oid));
                //  trace(&format!("new tree has {} entries", new_tree.len()));
                Self::describe_tree(&new_tree, "commit_merge: conflict");
                let message = "merge after fetch (conflict)";
                Ok(Some(self.write_commit(
                    new_tree,
                    message,
                    &[&our_commit, &their_commit],
                )?))
            }
        }
    }
    /** dump merge conflicts for debugging */
    fn dump_conflicts(&self, index: &Index) -> NullResult {
        trace(&format!(
            "index has {} conflicts",
            dump_error!(index.conflicts()).count()
        ));
        let mut desc_opts = DescribeOptions::new();
        desc_opts.show_commit_oid_as_fallback(true);
        for ic in dump_error!(index.conflicts()) {
            let icc = &(dump_error!(ic));
            //   trace(&format!("index conflict {:#?}", &icc));
            trace("conflict found");
            if let Some(a) = &icc.ancestor {
                let o = dump_error!(self.repo.find_object(a.id, None));
                trace(&format!("ancestor {:?} ", describe_git_object(&o)));
            //     dump_error!(o.describe(&desc_opts)).format(None)
            // ));
            } else {
                trace("no ancestor");
            }
            if let Some(o) = &icc.our {
                let o = dump_error!(self.repo.find_object(o.id, None));
                trace(&format!("ours {:?} ", describe_git_object(&o)));
            //  dump_error!(o.describe(&desc_opts)).format(None)
            // ));
            } else {
                trace("no ours");
            }
            if let Some(t) = &icc.their {
                let o = dump_error!(self.repo.find_object(t.id, None));
                trace(&format!("theirs {:?} ", describe_git_object(&o)));
            //     dump_error!(o.describe(&desc_opts)).format(None)
            // ));
            } else {
                trace("no theirs");
            }
            trace("conflict");
        }
        trace("conflicts listed");
        Ok(())
    }
    /** push changes to remote (push the local repo to a server) */
    pub fn push(
        &mut self, // url: &str,
        force: bool,
    ) -> NullResult {
        trace("preparing to push...");
        repo_timer!("push repo");
        {
            repo_trace!("pushing to remote");
            let mut remote: Remote<'_> = match self.try_get_remote() {
                Ok(r) => r,
                Err(_) => return Err(repo_error!("no remote")),
            };
            trace("authenticating......");
            let mut cb = git2::RemoteCallbacks::new();
            Self::set_remote_callbacks(
                &self
                    .ssh_path
                    .to_str()
                    .ok_or_else(|| repo_error!("bad path"))?
                    .to_owned(),
                self.slurp_ssh,
                &mut cb,
            )?;
            trace("connecting...");
            remote.connect_auth(Direction::Push, Some(cb), None)?;
            trace("connected, preparing...");
            trace("authenticating......");
            let mut cb = git2::RemoteCallbacks::new();
            Self::set_remote_callbacks(
                &self
                    .ssh_path
                    .to_str()
                    .ok_or_else(|| repo_error!("bad path"))?
                    .to_owned(),
                self.slurp_ssh,
                &mut cb,
            )?;
            let mut push_options = PushOptions::new();
            push_options.remote_callbacks(cb);
            trace("pushing...");
            let base_refspec = format!(
                "refs/heads/{}:refs/heads/{}",
                self.required_branch, self.required_branch
            );
            let refspec = (if force { "+" } else { "" }).to_owned() + &base_refspec;
            trace(&format!("actually pushing {})...", refspec.as_str(),));
            remote.push(&[refspec.as_str()], Some(&mut push_options))?;
            trace("actually pushed.");
            //     self.needs_push = false;
        }
        trace("after push, clearing needs push...");
        self.needs_push = false;
        Ok(())
    }
    /** set the remote callbacks for a repo access. In particular, set up the SSL credentials to access the repo. `ssh_path` is the file location of the SSH keys. If `slurp_ssh` is set then we try to read the files ourselves, otherwise we let Git try. The resulting callbacks are written to `cb`.

    TODO: pass in the keys instead of slurping them */
    fn set_remote_callbacks(
        ssh_path: &String,
        slurp_ssh: bool,
        cb: &mut RemoteCallbacks<'_>,
    ) -> NullResult {
        trace(&format!(
            "setting remote credentials, path {}, {}",
            ssh_path,
            if slurp_ssh { "SLURP" } else { "NO SLURP" }
        ));
        let git_config = git2::Config::open_default()?;
        let mut ch = CredentialHandler::new(git_config);
        let mut try_count: i8 = 0;
        const MAX_TRIES: i8 = 5;
        let ssh_path_copy = ssh_path.clone();
        cb.credentials(move |url, username, allowed| {
            trace(&format!(
                "try #{}: looking for credential (url {}, user {:?}, type {:?}) {}",
                &try_count,
                &url,
                &username,
                &allowed,
                if slurp_ssh { "SLURP" } else { "NO SLURP" }
            ));
            if allowed.contains(CredentialType::SSH_KEY) && !slurp_ssh {
                //  let ssh_path = format!("{}{}", SSL_KEY_FILE, ".pub");
                trace("trying ssh key credential");
                let username = username.expect("no user name");
                let copy2 = ssh_path_copy.clone();
                let privatekey = Path::new(&copy2);
                let copy3 = ssh_path_copy.clone();
                let publickey = PathBuf::from(format!("{}.pub", copy3));
                trace(&format!(
                    "no slurp: username is {}, public is {:?}, private is {:?}",
                    &username, &publickey, &privatekey
                ));
                let cred = Cred::ssh_key(username, Some(&publickey), &privatekey, None);
                trace("returning ssh key credential");
                return cred;
            }
            if allowed.contains(CredentialType::SSH_MEMORY) && slurp_ssh {
                trace("trying ssh memory credential");
                let username = username.expect("no user name");
                // let publickey_fn = PathBuf::from(format!("{}.pub", ssh_path_copy));
                // let publickey = slurp::read_all_to_string(publickey_fn).unwrap_or_else(|err| {
                //     trace(&format!("bad public key file: {:?}", err));
                //     panic!("bad");
                // });
                // trace(&format!("got public key, length {}", publickey.len()));

                let publickey = Self::try_slurp_file(
                    &PathBuf::from(format!("{}.pub", ssh_path_copy)),
                    "public key",
                )
                .unwrap();
                // let privatekey_fn = ssh_path_copy.clone();
                // let privatekey = slurp::read_all_to_string(privatekey_fn).unwrap_or_else(|err| {
                //     trace(&format!("bad private key file: {:?}", err));
                //     panic!("bad");
                // });
                // trace(&format!("got private key, length {}", privatekey.len()));

                let privatekey =
                    Self::try_slurp_file(&PathBuf::from(ssh_path_copy.clone()), "private key")
                        .unwrap();
                let cred = Cred::ssh_key_from_memory(username, Some(&publickey), &privatekey, None);
                trace("returning ssh memory credential");
                return cred;
            }
            trace(&format!(
                "look for credential {:?} ({} tries)",
                allowed, try_count
            ));
            try_count += 1;
            if try_count > MAX_TRIES {
                trace("too many tries for ssh key");
                panic!("too many ssh tries".to_string());
            }
            ch.try_next_credential(url, username, allowed)
        });
        cb.push_update_reference(|refer, reason_opt| {
            trace("pushing update reference");
            let msg = if let Some(reason) = reason_opt {
                format!("rejected because: {}", reason)
            } else {
                "accepted".to_string()
            };
            trace(&format!(
                "push update reference ({}): {}) done",
                &refer, &msg
            ));
            Ok(())
        });
        trace("set up credentials");
        Ok(())
    }
    /** `try_slurp_file` tries to slurp a file */
    fn try_slurp_file(path: &PathBuf, narr: &str) -> Result<String, RepoError> {
        trace(&format!("slurping from {:?} for {}...", &path, narr));
        match slurp::read_all_to_string(path) {
            Err(err) => {
                let msg = format!("bad {} file: {:?}", narr, err);
                trace(&msg);
                Err(RepoError::from(err))
            }
            Ok(s) => {
                trace(&format!("got {}", narr /* , &s */));
                Ok(s)
            }
        }
    }
    // /** Get a path to SSL credentials. */
    // fn ssh_path(file_name: &str) -> Result<PathBuf, RepoError> {
    //     trace(&format!("getting ssh path for {}...", file_name));
    //     let mut path = dump_error!(dirs::home_dir().ok_or_else(|| repo_error!("bad home dir")));
    //     path.push(".ssh");
    //     path.push(file_name);
    //     trace(&format!("ssh path is {:?}", path));
    //     Ok(path)
    // }

    /** apply a set of changes and commit them
     */
    pub fn apply_changes(&mut self, changes: &ChangeList) -> NullResult {
        repo_timer!("applying changes");
        // // let now = SystemTime::now();
        // // trace(&format!("applying {} changes", changes.len()));
        let changes_with_ords = self.add_oids_to_changelist(changes.to_vec());
        self.actually_do_changes(&changes_with_ords)?;
        trace("setting needs change...");
        self.set_needs_push();
        // trace2(&format!(
        //     "apply changes took {}s",
        //     now.elapsed()?.as_millis() as f64 / 1000.0
        // ));
        // trace("changes applied");
        Ok(())
    }
    /** notify a blob and get its [`RepoOid`]  */
    pub fn notify_blob(&mut self, content: &[u8]) -> Result<RepoOid, RepoError> {
        trace(&format!("blob is {}", String::from_utf8_lossy(&content)));
        Ok(RepoOid::from_oid(&dump_error!(self.repo.blob(content))))
    }
    /** Give a path, retrieve the blob at that location. */
    pub fn blob_from_path(&self, path: &str) -> Result<Vec<u8>, RepoError> {
        //        FUTURE: cache  the following and update each commit
        repo_trace!(&format!("getting blob (path {:?})...", path));
        let commit =
            dump_error!(self.find_last_commit()).ok_or_else(|| (repo_error!("no commit")))?;
        let tree = dump_error!(commit.tree());
        //   trace(&format!("commit tree has {} entries", tree.len()));
        Self::describe_tree(&tree, "commit_merge:commit");
        let subtree = self
            .try_get_subtree(tree)?
            .ok_or_else(|| repo_error!("no subtree"))?;
        Self::describe_tree(&subtree, "commit_merge:entries in subtree");
        trace("getting entry from repo...");
        let entry = dump_error!(subtree.get_path(&Path::new(path)));
        let id = dump_error!(self.repo.find_blob(entry.id()));
        let content = id.content();
        let content_length = content.len();
        let mut blob: Vec<u8> = vec![];
        blob.resize(content_length, 0);
        blob.clone_from_slice(content);
        trace(&format!("got blob, length {}.", content_length));
        Ok(blob)
    }
    /** do the changes */
    fn actually_do_changes(&mut self, changes: &ChangeWithOidList) -> NullResult {
        repo_trace!("actually doing changes");
        let old_parent_tree = self.get_latest_tree()?.clone();
        Self::describe_tree(&old_parent_tree, "actually_do_changes:old_parent_tree");
        trace("actually_do_changes: getting old subtree...");
        let old_subtree = self
            .try_get_subtree(old_parent_tree.clone())?
            .ok_or_else(|| repo_error!("no items dir"))?;
        Self::describe_tree(&old_subtree, "actually_do_changes:old subtree");
        trace("actually_do_changes: building new subtree...");
        let mut new_subtree_builder = dump_error!(self.repo.treebuilder(Some(&old_subtree)));
        let messages = dump_error!(Self::apply_changes_to_item_treebuilder(
            &mut new_subtree_builder,
            changes
        ));
        let new_subtree_oid = new_subtree_builder.write()?;
        trace(&format!(
            "actually_do_changes: new subtree {}",
            new_subtree_oid
        ));
        let new_subtree = dump_error!(self.repo.find_tree(new_subtree_oid));
        Self::describe_tree(&new_subtree, "actually_do_changes:new_subtree");
        let mut new_parent_treebuilder = dump_error!(self.repo.treebuilder(Some(&old_parent_tree)));
        Self::insert_directory(&mut new_parent_treebuilder, &self.item_dir, new_subtree_oid)?;
        let new_parent_tree_oid = dump_error!(new_parent_treebuilder.write());
        let new_parent_tree = dump_error!(self.repo.find_tree(new_parent_tree_oid));
        Self::describe_tree(&new_parent_tree, "actually_do_changes:new parent");
        trace(&format!(
            "actually_do_changes: new parent tree {}",
            new_parent_tree_oid
        ));
        trace("all changes applied, writing commit");
        let parent_commit = self
            .find_last_commit()?
            .ok_or_else(|| (repo_error!("no commit")))?;
        self.write_commit(new_parent_tree, &messages.join(" "), &[&parent_commit])?;
        trace("actually done changes.");
        Ok(())
    }
    /** apply the changes to a tree builder for items */
    fn apply_changes_to_item_treebuilder(
        tree_builder: &mut TreeBuilder<'_>,
        changes_with_oids: &ChangeWithOidList,
    ) -> Result<Vec<String>, RepoError> {
        repo_trace!("applying changes to tree builder");
        let messages: Vec<String> = changes_with_oids
            .iter()
            .map(|c: &ChangeWithOid| c.change.descr.clone())
            .collect();
        trace("applying changes...");
        for c in changes_with_oids.iter() {
            trace(&format!("applying change {:?} to tree...", &c.change));
            let path = Path::new(&c.change.path);
            match &c.change.op {
                ObjectOperation::Add(_data)
                | ObjectOperation::Modify(_data)
                | ObjectOperation::Fix(_data) => {
                    // let repoid = self.notify_blob(data.as_bytes())?;

                    let oid = c.oid.to_oid()?;
                    //   trace("actually_do_changes: j");
                    let entry = tree_builder.insert(path, oid, 0o100644 /* regular */);
                    match entry {
                        Err(e) => {
                            trace(&format!("insert error is {:?}", e));
                            return Err(repo_error!(&format!("git error: {:?}", e)));
                        }
                        Ok(e) => trace(&format!("entry is {:?}", e.id())),
                    }
                }
                ObjectOperation::Delete => tree_builder.remove(path)?,
                _ => {
                    return Err(repo_error!(&format!(
                        "change type {:?} not implemented",
                        c.change.op
                    )))
                }
            };
        }
        Ok(messages)
    }
    /** */
    pub fn add_oids_to_changelist(&mut self, changelist: ChangeList) -> ChangeWithOidList {
        changelist
            .iter()
            .map(|change| {
                let oid = match &change.op {
                    ObjectOperation::Add(data)
                    | ObjectOperation::Modify(data)
                    | ObjectOperation::Fix(data) => self
                        .notify_blob(data.as_bytes())
                        .expect("could not convert"),
                    _ => RepoOid::new(),
                };
                change.clone().with_oid(oid)
            })
            .collect()
    }
    /** find all the items in the repository */
    pub fn list_all(&self) -> Result<Vec<EntryDescr>, RepoError> {
        repo_trace!("listing all");
        let tree = self.get_latest_tree()?;
        trace(&format!("latest tree has {} entries", tree.len()));
        let subtree = self
            .try_get_subtree(tree)?
            .ok_or_else(|| repo_error!("no subtree"))?;
        trace("listing all - iterating...");
        trace(&format!("subtree has {} entries", subtree.len()));
        let all: Vec<EntryDescr> = subtree
            .iter()
            .map(|te| {
                trace(&format!("grabbing {:?} ({:?})", te.name(), te.kind()));
                EntryDescr {
                    oid: RepoOid::from_oid(&te.id()),
                    path: te.name().unwrap_or("??").to_string(),
                    kind: format!("{:?}", te.kind()),
                    blob: str::from_utf8(
                        te.to_object(&self.repo)
                            .expect("could not convert to object")
                            .peel_to_blob()
                            .expect("could not peel to blob")
                            .content(),
                    )
                    .expect("bad entry descr")
                    .to_owned(),
                }
            })
            .collect();
        trace(&format!("listed all, {} found.", all.len()));
        Ok(all)
    }
    /** print out some debug info about a tree */
    pub(crate) fn describe_tree(tree: &Tree<'_>, descr: &str) {
        let mut descrs: Vec<String> = vec![];
        descrs.push(format!(
            "tree: {} ({} entries) ",
            descr,
            tree.len(),
            //  tree.id()
        ));
        let mut i = 0;
        'entries: for it in tree.iter() {
            descrs.push(format!(
                "{:?} ({:?})",
                it.name().unwrap_or(""),
                it.kind().map_or("NONE".to_string(), |k| format!("{:?}", k)),
                //   it.id()
            ));
            i += 1;
            if i > 10 {
                descrs.push("...".to_string());
                break 'entries;
            }
        }
        trace(&descrs.join(" "));
    }
    /** item entry from index entry */
    fn item_entry_from_index_entry(&self, ie: &git2::IndexEntry) -> RepoResult<ItemEntry> {
        Ok(ItemEntry {
            path: String::from_utf8_lossy(&ie.path).to_string(),
            data: self.repo.find_blob(ie.id)?.content().to_vec(),
        })
    }
    /**  conflict from index conflict */
    fn conflict_from_index_conflict(&self, ic: &IndexConflict) -> RepoResult<Conflict> {
        Ok(Conflict {
            ancestor: match &ic.ancestor {
                Some(ie) => Some(self.item_entry_from_index_entry(&ie)?),
                None => None,
            },
            our: match &ic.our {
                Some(ie) => Some(self.item_entry_from_index_entry(&ie)?),
                None => None,
            },
            their: match &ic.their {
                Some(ie) => Some(self.item_entry_from_index_entry(&ie)?),
                None => None,
            },
        })
    }
    /** the conflicts, if any, resulting from the merge */
    pub fn conflicts(&self, mo: &MergeOutcome) -> RepoResult<ConflictList> {
        match mo {
            MergeOutcome::AlreadyUpToDate => Ok(vec![]),
            MergeOutcome::Merged(_i) => Ok(vec![]),
            MergeOutcome::Conflict(i) => {
                trace("finding conflicts");
                let mut cl: ConflictList = vec![];
                for conflict in i.conflicts()? {
                    let c = self.conflict_from_index_conflict(&conflict?)?;
                    cl.push(c);
                }
                Ok(cl)
            }
        }
    }
    /** a description identifying the repo for use in diagnostic
    traces */
    pub fn trace_descr(&self) -> String {
        format!(
            "repo with path: {:?}, url: {}",
            self.path,
            match &self.url {
                None => "(none)",
                Some(u) => &u,
            }
        )
    }
}
impl Drop for FanlingRepository {
    fn drop(&mut self) {
        trace("dropping repo");
        if thread::panicking() {
            trace("already panicking, so no more checks")
        } else {
            if self.needs_push {
                trace("repo needs push but dropping|");
            }
        }
    }
}
/** descibe a git2 Object without crashing */
fn describe_git_object(obj: &git2::Object<'_>) -> String {
    match obj.kind() {
        None => "(nothing)".to_owned(),
        Some(git2::ObjectType::Commit) => "commit".to_owned(),
        Some(k) => format!("other {:?}", k),
    }
}
/** a version of an `Item`, for merging */
//#[derive(Debug)]
pub struct ItemEntry {
    pub path: String,
    pub data: Vec<u8>,
}
impl fmt::Debug for ItemEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} ({})",
            self.path,
            str::from_utf8(&self.data).unwrap_or("(bad string)")
        )
    }
}

/**    a conflict that needs to be resolved (from a merge)*/
//#[derive(Debug)]
pub struct Conflict {
    pub ancestor: Option<ItemEntry>,
    pub our: Option<ItemEntry>,
    pub their: Option<ItemEntry>,
}
impl fmt::Debug for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Conflict [")?;
        match &self.ancestor {
            Some(ie) => write!(f, "ancestor: {:?} ", ie)?,
            _ => (),
        }
        match &self.our {
            Some(ie) => write!(f, "our: {:?} ", ie)?,
            _ => (),
        }
        match &self.their {
            Some(ie) => write!(f, "their: {:?} ", ie)?,
            _ => (),
        }
        write!(f, "]")
    }
}
/** a list of conflicts*/
pub type ConflictList = Vec<Conflict>;

/** possible outcomes of doing a fetch*/
pub enum MergeOutcome {
    AlreadyUpToDate,
    Merged(Index),
    Conflict(Index),
}
impl fmt::Debug for MergeOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyUpToDate => write!(f, "AlreadyUpToDate"),
            Self::Merged(_) => write!(f, "Merged"),
            Self::Conflict(_) => write!(f, "Conflict"),
        }
    }
}
impl MergeOutcome {
    /** are there any conflicts arising out of the merge? */
    pub fn has_conflict(&self) -> bool {
        if let Self::Conflict(_) = &self {
            true
        } else {
            false
        }
    }
    pub fn index(&self) -> Option<&Index> {
        match self {
            Self::AlreadyUpToDate => None,
            Self::Merged(ix) | Self::Conflict(ix) => Some(ix),
        }
    }
}
