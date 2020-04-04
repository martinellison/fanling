//* useful functions used in the tests
use super::*;
pub(crate) fn create_simple_action(name: &str) -> String {
    format!(
        r#"{{"t":"Simple","i":"","a":{{"Create":[{{"ident":"","type":"Simple"}},{{"name":"{}","text":"aaaa"}}]}}}}"#,
        name
    )
}
pub(crate) fn create_task_action(name: &str, text: &str) -> String {
    format!(
        r#"{{"t":"Task","i":"","a":{{"Create":[{{"ident":"","type":"Task"}},{{"name":"{}","text":"{}","priority":"10","context":"default_context","deadline":"1970-01-01 00:00:00","show_after_date":"1970-01-01 00:00:00"}}]}}}}"#,
        &name, &text,
    )
}
pub(crate) fn update_simple_action(ident: &str, name: &str, text: &str) -> String {
    format!(
        r#"{{"t":"Simple","i":"{}","a":{{"Update":[{{"ident":"{}","type":"Simple"}},{{"name":"{}","text":"{}"}}]}}}}"#,
        &ident, &ident, &name, &text,
    )
}
pub(crate) fn update_task_action(ident: &str, name: &str, text: &str) -> String {
    format!(
        r#"{{"t":"Task","i":"{}","a":{{"Update":[{{"ident":"{}","type":"Task"}},{{"name":"{}","text":"{}","priority":"10","context":"default_context","deadline":"2021-01-01 00:00:00","show_after_date":"2020-01-01 00:00:00"}}]}}}}"#,
        &ident, &ident, &name, &text,
    )
}
pub(crate) fn pull_push_and_shutdown(engine: &mut FanlingEngine) -> crate::shared::NullResult {
    engine.execute(r#"{"a":"Pull"}"#)?;
    let check_data = r#"{"a":"CheckData","i":"","t":""}"#;
    let _resp = engine.execute(&check_data)?;
    engine.execute(r#"{"a":{"Push":{"force":false}}}"#)?;
    engine.execute(r#"{"a":"Shutdown","i":"","t":""}"#)?;
    engine.handle_event(&fanling_interface::CycleEvent::StopPC)?;
    Ok(())
}
pub(crate) fn check_test_data(
    engine: &mut super::FanlingEngine,
    ident: &str,
    flag: &str,
    expect: &str,
) -> NullResult {
    let show1 = format!(r#"{{"t":"","i":"{}","a":"Show"}}"#, ident);
    let resp = engine.execute(&show1)?;
    let act = resp.get_test_data(flag);
    assert_eq!(
        expect, act,
        "item {}: {} should be {}",
        &ident, &flag, &expect,
    );
    Ok(())
}
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
        Some(t) => assert_eq!(
            expected_text, t,
            "wrong field '{}' value in test result (for '{}')",
            &field_key, &test_ident
        ),
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
