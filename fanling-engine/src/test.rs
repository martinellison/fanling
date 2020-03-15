/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! tests for engine */
use super::*;
use crate::fanling_interface::Engine;
use std::fs;
use std::path::PathBuf;
#[test]
fn simple() -> crate::shared::NullResult {
    trace("simple test: start");

    const TEST_DIR1: &str = "testfiles1";
    let (test_dir, database_path) = utils::init_files(TEST_DIR1, "test-no-name");
    let mut options = utils::simple_options(&test_dir, &database_path);
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
    // TODO more tests
    trace("simple test: done.");
    Ok(())
}
#[test]
fn local() -> crate::shared::NullResult {
    let local_test_cases = vec![LocalTestCase {
        narr: "simple".to_string(),
        create_action: r#"{"t":"Simple","i":"","a":{"Create":[{"ident":"","type":"Simple"},{"name":"aaa","text":"aaaa"}]}}"#.to_string(),
        expected_text_after_create: "aaaa".to_string(),
        update_action2: r#"{"t":"Simple","i":"aaa-o2","a":{"Update":[{"ident":"aaa-o2","type":"Simple"},{"name":"bbb","text":"bbbb"}]}}"# .to_string(),
        expected_text_after_update: "bbbb".to_string(),
        update_action3: r#"{"t":"Simple","i":"aaa-o2","a":{"Update":[{"ident":"aaa-o2","type":"Simple"},{"name":"bbb","text":"cccc"}]}}"# .to_string(),
        expected_text_at_end:  "ccccbbbb".to_string(),
        field_key: "text".to_string(),   test_ident:  "aaa-o2" .to_string(),
    }, LocalTestCase {
         narr: "task".to_string(),
        create_action: r#"{"t":"Task","i":"","a":{"Create":[{"ident":"","type":"Task"},{"name":"aaaa","text":"aaaaa","priority":"10","context":"default_context","deadline":"2021-01-01 00:00:00","show_after_date":"2020-01-01 00:00:00"}]}}"#.to_string(),
        expected_text_after_create: "aaaaa".to_string(),
        update_action2: r#"{"t":"Task","i":"2021-01-01-aaaa-o2","a":{"Update":[{"ident":"2021-01-01-aaaa-o2","type":"Task"},{"name":"aaaa","text":"bbbbb","priority":"10","context":"default_context","deadline":"2021-01-01 00:00:00","show_after_date":"2020-01-01 00:00:00"}]}}"# .to_string(),
        expected_text_after_update: "bbbbb".to_string(),
        update_action3: r#"{"t":"Task","i":"2021-01-01-aaaa-o2","a":{"Update":[{"ident":"2021-01-01-aaaa-o2","type":"Task"},{"name":"aaaa","text":"ccccc","priority":"10","context":"default_context","deadline":"2021-01-01 00:00:00","show_after_date":"2020-01-01 00:00:00"}]}}"# .to_string(),
        expected_text_at_end:  "cccccbbbbb".to_string(),
        field_key: "text".to_string(), test_ident:  "2021-01-01-aaaa-o2" .to_string(),
    }];
    for test_case in local_test_cases {
        local_test(&test_case)?
    }
    Ok(())
}
struct LocalTestCase {
    narr: String,
    create_action: String,
    expected_text_after_create: String,
    update_action2: String,
    expected_text_after_update: String,
    update_action3: String,
    expected_text_at_end: String,
    field_key: String,
    test_ident: String,
}
fn local_test(ltc: &LocalTestCase) -> crate::shared::NullResult {
    trace(&format!("local test: start {}", &ltc.narr));
    const TEST_DIR1: &str = "testfiles2";
    let (test_dir, database_path) = utils::init_files(TEST_DIR1, "test-local");
    let options = super::EngineOptions {
        repo_options: taipo_git_control::RepoOptions {
            path: PathBuf::from(&test_dir).into_boxed_path(),
            name: "tester".to_string(),
            email: "tester@example.com".to_string(),
            write_to_server: true,
            ..taipo_git_control::RepoOptions::default()
        },
        interface_type: super::InterfaceType::PC,
        search_options: crate::search::SearchOptions { database_path },
        uniq_pfx: "o".to_string(),
        auto_link: false,
    };
    {
        trace("local test: create item");
        let mut engine = super::FanlingEngine::new(&options)?;
        engine.execute(&ltc.create_action)?;
        engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
        dump_fanling_error!(utils::check_engine(
            &mut engine,
            &ltc.expected_text_after_create,
            &ltc.field_key,
            &ltc.test_ident
        ));
    }

    trace("local test: repo2 - update");
    let mut engine2 = utils::test_engine(TEST_DIR1, "test-local", "test-local2", "p")?;
    engine2.execute(&ltc.update_action2)?;
    trace("local test: repo3 - update");
    let mut engine3 = utils::test_engine(TEST_DIR1, "test-local", "test-local3", "p")?;
    engine3.execute(&ltc.update_action3)?;

    trace("local test: repo2 - push");
    engine2.execute(r#"{"a":"Pull"}"#)?;
    engine2.execute(r#"{"a":{"Push":{"force":false}}}"#)?;
    engine2.execute(r#"{"a":"Shutdown","i":"","t":""}"#)?;
    engine2.handle_event(&fanling_interface::CycleEvent::StopPC)?;
    dump_fanling_error!(utils::check_engine(
        &mut engine2,
        &ltc.expected_text_after_update,
        &ltc.field_key,
        &ltc.test_ident
    ));
    {
        trace("local test: check push from repo2");
        let mut engine = super::FanlingEngine::new(&options)?;
        dump_fanling_error!(utils::check_engine(
            &mut engine,
            &ltc.expected_text_after_update,
            &ltc.field_key,
            &ltc.test_ident
        ));
    }

    trace("local test: repo3 - push");
    engine3.execute(r#"{"a":"Pull"}"#)?;
    engine3.execute(r#"{"a":{"Push":{"force":false}}}"#)?;
    engine3.execute(r#"{"a":"Shutdown","i":"","t":""}"#)?;
    engine3.handle_event(&fanling_interface::CycleEvent::StopPC)?;
    dump_fanling_error!(utils::check_engine(
        &mut engine2,
        &ltc.expected_text_after_update,
        &ltc.field_key,
        &ltc.test_ident
    ));
    {
        trace("local test: check push from repo");
        let mut engine = super::FanlingEngine::new(&options)?;
        dump_fanling_error!(utils::check_engine(
            &mut engine,
            &ltc.expected_text_at_end,
            &ltc.field_key,
            &ltc.test_ident
        ));
    }
    trace("local test: done.");
    Ok(())
}

struct CloneTestCase {
    narr: String,
    create_action: String,
    clone_action: String,
    expected_text: String,
    field_key: String,
    test_ident: String,
}
#[test]
fn clone() -> crate::shared::NullResult {
    trace("clone test: start");
    let test_cases = vec![CloneTestCase {
        narr: "simple".to_string(),
        create_action: r#"{"t":"Simple","i":"","a":{"Create":[{"ident":"","type":"Simple"},{"name":"aaa","text":"aaaa"}]}}"#.to_string(),
        clone_action: r#"{"t":"Simple","i":"aaa-a2","a":"Clone"}"#.to_string(),
        expected_text: "aaaa".to_string(),
        field_key: "text".to_string(),   test_ident:  "aaa-b3" .to_string(),
    },
         CloneTestCase {
        narr: "task".to_string(),
        create_action: r#"{"t":"Task","i":"","a":{"Create":[{"ident":"","type":"Task"},{"name":"aaaa","text":"aaaaa","priority":"10","context":"default_context","deadline":"2021-01-01 00:00:00","show_after_date":"2020-01-01 00:00:00"}]}}"#.to_string(),
        clone_action: r#"{"t":"Task","i":"2021-01-01-aaaa-a2","a":"Clone"}"#.to_string(),
        expected_text: "aaaaa".to_string(),
        field_key: "text".to_string(),   test_ident:  "2021-01-01-aaaa-b3" .to_string(),
    },
    ];
    for tc in test_cases {
        trace(&format!("clone test: start {}", &tc.narr));
        const TEST_DIR1: &str = "testfiles3";
        let (test_dir, database_path) = utils::init_files(TEST_DIR1, "test-clone");
        let mut options = utils::simple_options(&test_dir, &database_path);
        {
            trace("clone test: first engine - creating");
            let mut engine = super::FanlingEngine::new(&options)?;
            trace("clone test: first engine - executing create item");
            let response = engine.execute(&tc.create_action)?;
            trace(&format!("{}: response {:?}", &tc.test_ident, &response));
        }
        {
            trace("clone test: clone");
            options.uniq_pfx = "b".to_string();
            let mut engine = super::FanlingEngine::new(&options)?;
            let response = engine.execute(&tc.clone_action)?;
            assert_eq!(
                Some(tc.test_ident.clone()),
                response.get_ident(),
                "{}: ident missing or wrong, response is {:#?}",
                &tc.narr,
                &response
            );
            dump_fanling_error!(utils::check_engine(
                &mut engine,
                &tc.expected_text,
                &tc.field_key,
                &tc.test_ident
            ));
            let _response = engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
        }
    }
    Ok(())
}
#[test]
fn no_name() -> crate::shared::NullResult {
    trace("no name test: start");
    let create_actions: Vec<String> = vec![ r#"{"t":"Simple","i":"","a":{"Create":[{"ident":"","type":"Simple"},{"name":"","text":"aaaa"}]}}"#.to_string(),
               r#"{"t":"Task","i":"","a":{"Create":[{"ident":"","type":"Task"},{"name":"","text":"","priority":"10","context":"default_context","deadline":"1970-01-01 00:00:00","show_after_date":"1970-01-01 00:00:00"}]}}"#.to_string(),
    ];
    let mut i = 0;
    for ca in create_actions {
        trace(&format!("no name case {}", i));
        const TEST_DIR1: &str = "testfiles4";
        let (test_dir, database_path) = utils::init_files(TEST_DIR1, "test-no-name");
        let options = utils::simple_options(&test_dir, &database_path);
        let mut engine = super::FanlingEngine::new(&options)?;
        let resp = engine.execute(&ca)?;
        // trace(&format!("result of execution is {:#?}", resp));
        assert!(
            resp.num_tags() > 0,
            "execution response should have tags {:#?}",
            resp
        );
        i += 1;
    }
    Ok(())
}
#[test]
///  tests for task ready including blocking
fn ready_task() -> crate::shared::NullResult {
    trace("ready task test: start");
    const TEST_DIR1: &str = "testfiles5";
    let (test_dir, database_path) = utils::init_files(TEST_DIR1, "test-no-name");
    let options = utils::simple_options(&test_dir, &database_path);
    let mut engine = super::FanlingEngine::new(&options)?;
    let create1 = r#"{"t":"Task","i":"","a":{"Create":[{"ident":"","type":"Task"},{"name":"t1","text":"task 1","priority":"10","context":"default_context","deadline":"1970-01-01 00:00:00","show_after_date":"1970-01-01 00:00:00"}]}}"#.to_string();
    let resp = engine.execute(&create1)?;
    let ident1 = resp.get_ident().unwrap();
    let create2 = r#"{"t":"Task","i":"","a":{"Create":[{"ident":"","type":"Task"},{"name":"t2","text":"task 2","priority":"10","context":"default_context","deadline":"1970-01-01 00:00:00","show_after_date":"1970-01-01 00:00:00"}]}}"#.to_string();
    let resp = engine.execute(&create2)?;
    let ident2 = resp.get_ident().unwrap();
    let block = format!(
        r#"{{"t":"Task","i":{:?},"a":{{"BlockBy":{:?}}}}}"#,
        &ident1, &ident2
    );
    let _resp = engine.execute(&block)?;
    Ok(())
}
/// useful functions used in the tests
mod utils {
    use super::*;
    pub(crate) fn simple_options(test_dir: &str, database_path: &str) -> super::EngineOptions {
        super::EngineOptions {
            repo_options: taipo_git_control::RepoOptions {
                path: PathBuf::from(test_dir).into_boxed_path(),
                name: "tester".to_string(),
                email: "tester@example.com".to_string(),
                write_to_server: true,
                ..taipo_git_control::RepoOptions::default()
            },
            interface_type: super::InterfaceType::PC,
            search_options: crate::search::SearchOptions {
                database_path: database_path.to_string(),
            },
            uniq_pfx: "a".to_string(),
            auto_link: false,
        }
    }
    pub(crate) fn init_files(dir: &str, subdir: &str) -> (String, String) {
        trace(&"initialising files...".to_string());
        // const TEST_DIR1: &str = "testfiles2";
        let test_dir = format!("{}/{}", dir, subdir);
        let database_path = format!("{}.db", test_dir);
        let _ = fs::remove_dir_all(&test_dir);
        let _ = fs::remove_file(&database_path);
        let _ = fs::remove_dir_all(dir);
        let _ = fs::create_dir_all(dir);
        (test_dir, database_path)
    }
    pub(crate) fn test_engine(
        test_dir_base: &str,
        old_name: &str,
        name: &str,
        uniq_pfx: &str,
    ) -> crate::shared::FLResult<crate::FanlingEngine> {
        trace(&format!(
            "local repo: base: {} old name: {} name: {} prefix: {}",
            test_dir_base, old_name, name, uniq_pfx
        ));
        let url = format!("{}/{}", test_dir_base, old_name);
        let test_dir2 = format!("{}/{}", test_dir_base, name);
        let database2_path = format!("{}.db", test_dir2);
        let _ = fs::remove_dir_all(&test_dir2);
        let _ = fs::remove_file(&database2_path);

        let options = super::EngineOptions {
            repo_options: taipo_git_control::RepoOptions {
                url: Some(url),
                path: PathBuf::from(test_dir2).into_boxed_path(),
                name: "tester".to_string(),
                email: "tester@example.com".to_string(),
                write_to_server: true,
                ..taipo_git_control::RepoOptions::default()
            },
            interface_type: super::InterfaceType::PC,
            search_options: crate::search::SearchOptions {
                database_path: database2_path,
            },
            uniq_pfx: uniq_pfx.to_string(),
            auto_link: false,
        };

        let engine = super::FanlingEngine::new(&options)?;
        Ok(engine)
    }
    pub(crate) fn check_engine(
        engine: &mut crate::FanlingEngine,
        expected_text: &str,
        field_key: &str,
        test_ident: &str,
    ) -> crate::shared::NullResult {
        trace(&format!("testing engine ({})", engine.trace_descr()));
        //  let ident = "aaa-o2";
        let (item_base, item_values) = engine.world.get_item_parts(&test_ident.to_owned())?;
        assert_eq!(test_ident, item_base.ident);
        match item_values.get(field_key) {
            None => {
                return Err(fanling_error!(&format!(
                    "no '{}' value for item",
                    field_key
                )))
            }
            Some(t) => assert_eq!(expected_text, t),
        }
        Ok(())
    }

    /** convenience function for test traces */
    pub(crate) fn trace(txt: &str) {
        println!(
            "engine test {} {} {}",
            ansi_term::Colour::Red
                .bold()
                .blink()
                .on(ansi_term::Colour::Black)
                .paint(">".repeat(16)),
            ansi_term::Colour::Black
                .bold()
                .on(ansi_term::Colour::White)
                .paint(txt),
            ansi_term::Colour::Red
                .bold()
                .blink()
                .on(ansi_term::Colour::Black)
                .paint("<".repeat(16)),
        );
    }
}
