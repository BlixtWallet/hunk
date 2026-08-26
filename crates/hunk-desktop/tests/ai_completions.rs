use hunk_desktop::{composer_completions, prompt_after_completion};

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

#[test]
fn slash_menu_matches_the_retained_gpui_commands() {
    let items = composer_completions("/", 1, false, &[]);
    let names = items
        .iter()
        .map(|item| item.value.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "code",
            "plan",
            "review",
            "fast-mode-on",
            "fast-mode-off",
            "status",
            "login",
            "logout",
        ]
    );

    let status = composer_completions("/st", 3, false, &[]);
    assert_eq!(
        status.first().map(|item| item.value.as_str()),
        Some("status")
    );
}

#[test]
fn running_turn_disables_commands_that_mutate_the_session() {
    let items = composer_completions("/", 1, true, &[]);

    assert!(
        items
            .iter()
            .find(|item| item.value == "plan")
            .is_some_and(|item| item.disabled)
    );
    assert!(
        items
            .iter()
            .find(|item| item.value == "status")
            .is_some_and(|item| !item.disabled)
    );
}

#[test]
fn file_menu_prefers_matching_file_names_and_limits_results() {
    let paths = vec![
        "src/lib.rs".to_owned(),
        "src/main.rs".to_owned(),
        "tests/main.rs".to_owned(),
        "infra/main.tf".to_owned(),
        "examples/main.rs".to_owned(),
        "nested/example_main.rs".to_owned(),
        "README.md".to_owned(),
    ];

    let items = composer_completions("@mai", 4, false, paths.as_slice());

    assert_eq!(items.len(), 5);
    assert_eq!(items[0].label, "main.rs");
    assert!(items.iter().all(|item| item.kind == "file"));
    assert!(items.iter().all(|item| item.value.contains("main")));
}

#[test]
fn utf16_cursor_positions_work_after_non_bmp_characters() {
    let prompt = "🙂 inspect @mai";
    let paths = vec!["src/main.rs".to_owned(), "src/lib.rs".to_owned()];

    let items = composer_completions(prompt, utf16_len(prompt), false, paths.as_slice());

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].value, "src/main.rs");
}

#[test]
fn accepting_file_completion_replaces_only_the_active_token() {
    let prompt = "Compare @read with the current code";
    let cursor = utf16_len("Compare @read");

    let next = prompt_after_completion(prompt, cursor, "file", "docs/read me.md")
        .expect("file completion should apply");

    assert_eq!(next, "Compare \"docs/read me.md\" with the current code");
}

#[test]
fn accepting_slash_command_removes_the_command_token() {
    let next = prompt_after_completion("  /plan explain this", 7, "command", "plan")
        .expect("slash command should apply");

    assert_eq!(next, "explain this");
}
