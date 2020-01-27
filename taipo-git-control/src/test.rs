/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! tests for git repository support */

use crate::rand::Rng;
use crate::repo::FanlingRepository;
use crate::RepoOptions;

use std::path::PathBuf;

#[test]
fn it_works() {
    assert_eq!(IDENT_LENGTH as usize, rand_ident().len());
}
#[test]
fn blob() -> super::NullResult {
    let opts = RepoOptions {
        path: temp_repo_path().into_boxed_path(),
        name: "tester".to_string(),
        email: "m,e@acm.org".to_string(),
        url: None,
        item_dir: "items".to_string(),
        required_branch: Some("master".to_string()),
        required_remote: Some("origin".to_string()),
        write_to_server: false,
        ssh_path: PathBuf::from("??").into_boxed_path(),
        slurp_ssh: false,
    };
    let mut repo = FanlingRepository::new_open(&opts)?.0;
    let blob = &vec![];
    let _oid = repo.notify_blob(blob)?;
    // let blob2 = repo.get_blob(&oid)?;
    // assert_eq!(
    //     *blob, blob2,
    //     "retrieved blob does not match original, oid is {:?}",
    //     &oid
    // );
    Ok(())
}
#[test]
fn init() -> super::NullResult {
    let opts = RepoOptions {
        path: temp_repo_path().into_boxed_path(),
        name: "tester".to_string(),
        email: "m,e@acm.org".to_string(),
        url: None,
        item_dir: "items".to_string(),
        required_branch: Some("master".to_string()),
        required_remote: Some("origin".to_string()),
        write_to_server: false,
        ssh_path: PathBuf::from("??").into_boxed_path(),
        slurp_ssh: false,
    };
    let repo = FanlingRepository::new_open(&opts)?.0;
    trace(&format!("after init, repo state {}", repo.state()));
    Ok(())
}
#[test]
fn open() -> super::NullResult {
    let opts = RepoOptions {
        path: temp_repo_path().into_boxed_path(),
        name: "tester".to_string(),
        email: "m,e@acm.org".to_string(),
        url: None,
        item_dir: "items".to_string(),
        required_branch: Some("master".to_string()),
        required_remote: Some("origin".to_string()),
        write_to_server: false,
        ssh_path: PathBuf::from("??").into_boxed_path(),
        slurp_ssh: false,
    };
    let repo = FanlingRepository::new_open(&opts)?.0;
    trace(&format!("after init, repo state {}", repo.state()));
    //   repo.needs_push = false;
    let repo = FanlingRepository::new_open(&opts)?.0;
    trace(&format!("after open, repo state {}", repo.state()));
    // repo.needs_push = false;
    Ok(())
}
// #[test]
// /** this test needs repo to exist on disk */
// fn open_existing() -> super::NullResult {
//     trace("starting open existing...");
//     let opts = RepoOptions {
//         path: PathBuf::from("testfiles/testrep2.git").into_boxed_path(),
//         name: "tester".to_string(),
//         email: "m.e@acm.org".to_string(),
//         url: Some("git@test.jennyemily.hk:fanling/testrep2.git".to_owned()),
//         item_dir: "items".to_string(),
//         required_branch: Some("master".to_string()),
//         required_remote: Some("origin".to_string()),
//         write_to_server: true,
//     };
//     let mut repo = FanlingRepository::new_open(&opts)?.0;
//     trace("fetching...");
//     let fetch_result = repo.fetch()?;
//     trace(&format!(
//         "fetch result was {:?}; after fetch, repo state is {}; writing new file...",
//         fetch_result,
//         repo.state()
//     ));
//     match fetch_result {
//         MergeOutcome::AlreadyUpToDate => {}
//         MergeOutcome::Merged | MergeOutcome::Conflict(_) => {
//             let conflicts = repo.conflicts(fetch_result);
//             unimplemented!() /* TODO  * conflicts */
// ;
//             repo.set_needs_push();
//             repo.commit_merge(
//                 repo.repo.find_tree(merged_tree.to_oid()?)?,
//                 "test merge",
//                 &[
//                     &repo.repo.find_commit(ours.to_oid()?)?,
//                     &repo.repo.find_commit(theirs.to_oid()?)?,
//                 ],
//             )?;
//         }
//     }
//     let old_commit = repo
//         .latest_commit()?
//         .ok_or_else(|| (repo_error!("no commit")))?;
//     let file_name = rand_ident();
//     trace(&format!(
//         "written file {:?} in {:?}, committing...",
//         &file_name,
//         &repo.repo.find_commit(old_commit.to_oid()?)?
//     ));
//     // {
//     //  let commit = repo.find_last_commit()?;
//     //  FanlingRepository::display_commit(&repo.repo.find_commit(old_commit.to_oid()?)?);
//     // }
//     let mut cl: crate::ChangeList = vec![];
//     let id = repo.notify_blob(rand_text(64).as_bytes())?;
//     cl.push(crate::Change {
//         op: crate::ObjectOperation::Add(id),
//         path: file_name.clone(),
//         descr: "test commit".to_owned(),
//     });
//     repo.apply_changes(&cl)?;
//     trace(&format!(
//         "after commit, repo state {}, pushing...",
//         repo.state()
//     ));
//     repo.push("", false)?;
//     trace(&format!("after push, repo state {}, done.", repo.state()));
//     let changes = repo.changes_since(old_commit)?;
//     trace(&format!("changes since commit {:#?}", &changes));
//     let oid = repo.oid_from_path(&file_name)?;
//     let _data = repo.get_blob(&oid);
//     assert!(
//         repo.list_all()?.len() > 0,
//         "no items found in the repository"
//     );
//     Ok(())
// }

#[test]
fn clone() -> super::NullResult {
    let _path = temp_repo_path();
    const URL_STRING: &str = &"git@test.jennyemily.hk:fanling/testrep.git";
    let opts = RepoOptions {
        path: temp_repo_path().into_boxed_path(),
        name: "tester".to_string(),
        email: "m,e@acm.org".to_string(),
        url: Some(URL_STRING.to_owned()),
        item_dir: "items".to_string(),
        required_branch: Some("master".to_string()),
        required_remote: Some("origin".to_string()),
        write_to_server: true,
        ssh_path: PathBuf::from("/tmp/id_rsa").into_boxed_path(),
        slurp_ssh: false,
    };
    let repo = FanlingRepository::clone_repo(&opts)?;
    trace(&format!("after clone, repo state {}", repo.state()));
    Ok(())
}

fn temp_repo_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("gittest-".to_string() + &rand_ident());
    path
}
const IDENT_CHAR: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
const IDENT_LENGTH: u8 = 8;
fn rand_ident() -> String {
    rand_text(IDENT_LENGTH)
}
fn rand_text(l: u8) -> String {
    (0..l)
        .map(|_n| {
            let rnum = rand::thread_rng().gen_range(0, IDENT_CHAR.len());
            IDENT_CHAR.char_indices().nth(rnum).unwrap().1
        })
        .collect()
}

fn trace(txt: &str) {
    println!("git {}", ansi_term::Colour::Green.paint(txt));
}
