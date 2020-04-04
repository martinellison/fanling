/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! tests for engine */
use super::*;
use crate::fanling_interface::Engine;
use std::fs;
use std::path::PathBuf;
#[cfg(test)]
mod utils;
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
        let _response = engine.execute(&utils::create_simple_action("aaa"))?;
        let _response = engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
    }
    {
        trace("simple test: second engine");
        options.uniq_pfx = "b".to_string();
        let mut engine = super::FanlingEngine::new(&options)?;
        let _response = engine.execute(&utils::update_simple_action("aaa-a2", "bbb", "bbbb"))?;
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
    // FUTURE more tests
    trace("simple test: done.");
    Ok(())
}
#[test]
fn local() -> crate::shared::NullResult {
    let local_test_cases = vec![
        LocalTestCase {
            narr: "simple".to_string(),
            create_action: utils::create_simple_action("aaa"),
            expected_text_after_create: "aaaa".to_string(),
            update_action2: utils::update_simple_action("aaa-o2", "bbb", "bbbb"),
            expected_text_after_update: "bbbb".to_string(),
            update_action3: utils::update_simple_action("aaa-o2", "bbb", "cccc"),
            expected_text_at_end: "ccccbbbb".to_string(),
            field_key: "text".to_string(),
            test_ident: "aaa-o2".to_string(),
        },
        LocalTestCase {
            narr: "task".to_string(),
            create_action: utils::create_task_action("aaaa", "aaaaa"),
            expected_text_after_create: "aaaaa".to_string(),
            update_action2: utils::update_task_action("aaaa-o2", "aaaa", "bbbbb"),
            expected_text_after_update: "bbbbb".to_string(),
            update_action3: utils::update_task_action("aaaa-o2", "aaaa", "ccccc"),
            expected_text_at_end: "cccccbbbbb".to_string(),
            field_key: "text".to_string(),
            test_ident: "aaaa-o2".to_string(),
        },
    ];
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
    fanling_trace!(&format!("local test: {}", &ltc.narr));
    const TEST_DIR1: &str = "testfiles2";
    let (test_dir, database_path) = utils::init_files(TEST_DIR1, "test-local");
    let mut options = utils::simple_options(&test_dir, &database_path);
    options.uniq_pfx = "o".to_string();
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
    let mut engine3 = utils::test_engine(TEST_DIR1, "test-local", "test-local3", "q")?;
    engine3.execute(&ltc.update_action3)?;

    trace("local test: repo2 - push");
    utils::pull_push_and_shutdown(&mut engine2)?;
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
    utils::pull_push_and_shutdown(&mut engine3)?;
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
    let test_cases = vec![
        CloneTestCase {
            narr: "simple".to_string(),
            create_action: utils::create_simple_action("aaa"),
            clone_action: r#"{"t":"Simple","i":"aaa-a2","a":"Clone"}"#.to_string(),
            expected_text: "aaaa".to_string(),
            field_key: "text".to_string(),
            test_ident: "aaa-b3".to_string(),
        },
        CloneTestCase {
            narr: "task".to_string(),
            create_action: utils::create_task_action("aaaa", "aaaaa"),
            clone_action: r#"{"t":"Task","i":"aaaa-a2","a":"Clone"}"#.to_string(),
            expected_text: "aaaaa".to_string(),
            field_key: "text".to_string(),
            test_ident: "aaaa-b3".to_string(),
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
                tc.test_ident.clone(),
                response.get_test_data("ident"),
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
            let check_data = r#"{"a":"CheckData","i":"","t":""}"#;
            let _resp = engine.execute(&check_data)?;
            let _response = engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
        }
    }
    Ok(())
}
#[test]
fn no_name() -> crate::shared::NullResult {
    trace("no name test: start");
    let create_actions: Vec<String> = vec![
        utils::create_simple_action(""),
        utils::create_task_action("", ""),
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
    let (test_dir, database_path) = utils::init_files(TEST_DIR1, "test-ready");
    let options = utils::simple_options(&test_dir, &database_path);
    let mut engine = super::FanlingEngine::new(&options)?;
    let create1 = utils::create_task_action("t1", "task 1");
    let resp = engine.execute(&create1)?;
    let ident1 = resp.get_test_data("ident");
    utils::check_test_data(&mut engine, &ident1, "ready", "true")?;
    let create2 = utils::create_task_action("t2", "task 2");
    let resp = engine.execute(&create2)?;
    let ident2 = resp.get_test_data("ident");
    utils::check_test_data(&mut engine, &ident2, "ready", "true")?;
    let ready = r#"{"a":"ListReady","i":"","t":""}"#;
    let resp = engine.execute(&ready)?;
    assert_eq!("3", resp.get_test_data("count"));
    let block = format!(
        r#"{{"t":"Task","i":{:?},"a":{{"BlockBy":{:?}}}}}"#,
        &ident1, &ident2
    );
    let unblock = format!(
        r#"{{"t":"Task","i":{:?},"a":{{"UnblockBy":{:?}}}}}"#,
        &ident1, &ident2
    );
    let _resp = engine.execute(&block)?;
    utils::check_test_data(&mut engine, &ident1, "ready", "false")?;
    let resp = engine.execute(&ready)?;
    assert_eq!("2", resp.get_test_data("count"));

    trace("ready test: repo2 - update");
    let mut engine2 = utils::test_engine(TEST_DIR1, "test-ready", "test-ready2", "p")?;
    utils::check_test_data(&mut engine2, &ident1, "ready", "false")?;
    let _resp = engine.execute(&unblock)?;
    utils::check_test_data(&mut engine, &ident1, "ready", "true")?;
    let resp = engine.execute(&ready)?;
    assert_eq!("3", resp.get_test_data("count"));
    //  TODO more testing: push to server and pull to second server
    //  TODO more testing: check in second repo for ready status
    let _resp = engine.execute(&block)?;
    let close_action = format!(r#"{{"t":"Task","i":"{}","a":"Close"}}"#, ident2);
    let _response = engine.execute(&close_action)?;
    utils::check_test_data(&mut engine, &ident1, "ready", "true")?;
    let resp = engine.execute(&ready)?;
    assert_eq!("2", resp.get_test_data("count"));
    //  TODO more testing: push to server and pull to second server
    // TODO more testing: check in second repo for ready status
    let check_data = r#"{"a":"CheckData","i":"","t":""}"#;
    let _resp = engine.execute(&check_data)?;
    Ok(())
}
