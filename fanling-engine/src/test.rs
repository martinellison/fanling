/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! tests for engine */
use crate::fanling_interface::Engine;
use std::fs;
use std::path::PathBuf;
// #[test]
// fn it_works() -> crate::shared::NullResult {
//     let options = super::EngineOptions {
//         repo_options: taipo_git_control::RepoOptions {
//             ..taipo_git_control::RepoOptions::default()
//         },
//         interface_type: super::InterfaceType::PC,
//         search_options: crate::search::SearchOptions {
//             database_path: "testdb.db".to_string(),
//         },
//         uniq_pfx: "test".to_string(),
//         auto_link: false,
//     };
//     let _engine = super::FanlingEngine::new(&options);
//     Ok(())
// }
#[test]
fn simple() -> crate::shared::NullResult {
    trace("simple test: start");
    const TEST_DIR1: &'static str = "testfiles1";
    let test_dir = format!("{}/{}",TEST_DIR1, "test-simple");
    let database_path = format!("{}.db", test_dir);
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::remove_file(&database_path);
    let _ = fs::remove_dir_all(&TEST_DIR1);
    let _ = fs::create_dir_all(&TEST_DIR1);
    let mut options = super::EngineOptions {
        repo_options: taipo_git_control::RepoOptions {
            path: PathBuf::from(test_dir).into_boxed_path(),
            name: "tester".to_string(),
            email: "tester@example.com".to_string(),
            ..taipo_git_control::RepoOptions::default()
        },
        interface_type: super::InterfaceType::PC,
        search_options: crate::search::SearchOptions {
            database_path: database_path,
        },
        uniq_pfx: "a".to_string(),
        auto_link: false,
    };
    {
        trace("simple test: first engine - creating");
        let mut engine = super::FanlingEngine::new(&options)?;
        trace("simple test: first engine - executing create item");
        let _response =     engine.execute( r#"{"t":"Simple","i":"","a":{"Create":[{"ident":"","type":"Simple"},{"name":"aaa","text":"aaaa"}]}}"# )?;
        let _response = engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
    }
    {
        trace("simple test: second engine");
        options.uniq_pfx = "b".to_string();
        let mut engine = super::FanlingEngine::new(&options)?;
        let _response =      engine.execute( r#"{"t":"Simple","i":"aaa-a2","a":{"Update":[{"ident":"aaa-a2","type":"Simple"},{"name":"bbb","text":"bbbb"}]}}"# )?;
        let _response = engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
    }
    {
        trace("simple test: third engine");
        options.uniq_pfx = "c".to_string();
        let mut engine = super::FanlingEngine::new(&options)?;
        let response = engine.execute(r#"{"t":"","i":"aaa-a2","a":"Show"}"#)?;
        //     trace(&format!("response is {:?}", response));
        assert_eq!(response.num_tags(), 2);
        assert_eq!(response.get_tag(0).0, "content");
        assert_eq!(response.get_tag(1).0, "always");
        assert!(!response.is_shutdown_required());
        let response = engine.execute(r#"{"a":"Shutdown","i":"","t":""}"#)?;
        assert!(response.is_shutdown_required());
    }
    // TODO more
    trace("simple test: done.");
    Ok(())
}
#[test]
fn local() -> crate::shared::NullResult {
    trace("local test: start");
    const TEST_DIR1: &'static str = "testfiles2";
    let test_dir = format!("{}/{}",TEST_DIR1, "test-local");
    let database_path = format!("{}.db", test_dir);
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::remove_file(&database_path);
    let _ = fs::remove_dir_all(&TEST_DIR1);
    let _ = fs::create_dir_all(&TEST_DIR1);
      let mut options = super::EngineOptions {
        repo_options: taipo_git_control::RepoOptions {
            path: PathBuf::from(&test_dir).into_boxed_path(),
            name: "tester".to_string(),
            email: "tester@example.com".to_string(),
            ..taipo_git_control::RepoOptions::default()
        },
        interface_type: super::InterfaceType::PC,
        search_options: crate::search::SearchOptions {
            database_path: database_path,
        },
        uniq_pfx: "o".to_string(),
        auto_link: false,
    };
    {
        let mut engine = super::FanlingEngine::new(&options)?;
        engine.execute( r#"{"t":"Simple","i":"","a":{"Create":[{"ident":"","type":"Simple"},{"name":"aaa","text":"aaaa"}]}}"# )?;
        engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
    }

    {
         let test_dir2 = format!("{}/{}",TEST_DIR1, "test-simple");
        let database2_path = format!("{}.db", test_dir2);
        let _ = fs::remove_dir_all(&test_dir2);
        let _ = fs::remove_file(&database2_path);

        let options2 = super::EngineOptions {
            repo_options: taipo_git_control::RepoOptions {
                url: Some(format!("{}", &test_dir)),
                path: PathBuf::from(test_dir2).into_boxed_path(),
                name: "tester".to_string(),
                email: "tester@example.com".to_string(),
                ..taipo_git_control::RepoOptions::default()
            },
            interface_type: super::InterfaceType::PC,
            search_options: crate::search::SearchOptions {
                database_path: database2_path,
            },
            uniq_pfx: "p".to_string(),
            auto_link: false,
        };

        let mut engine = super::FanlingEngine::new(&options2)?;
        engine.execute( r#"{"t":"Simple","i":"aaa-o2","a":{"Update":[{"ident":"aaa-o2","type":"Simple"},{"name":"bbb","text":"bbbb"}]}}"# )?;
        engine.execute(r#"{"a":{"Push":{"force":true}}}"#)?;
        engine.execute(r#"{"a":"Shutdown","i":"","t":""}"#)?;
        engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
    }
    {
        options.uniq_pfx = "q".to_string();
        let _engine = super::FanlingEngine::new(&options)?;
    }
    // TODO more
    trace("local test: done.");
    Ok(())
}
/** convenience function for test traces */
pub(crate) fn trace(txt: &str) {
    println!(
        "engine test {}",
        ansi_term::Colour::Black
            .on(ansi_term::Colour::Yellow)
            .paint(txt)
    );
}
