use std::collections::BTreeSet;

use hunk_codex::state::{AiState, ThreadLifecycleStatus, ThreadSummary, TurnStatus, TurnSummary};
use hunk_qt::{AiThreadCatalogProjection, AiThreadItem, AiThreadListModel};
use qtbridge::{QListModel, QObjectHolder};

fn thread(
    id: &str,
    title: Option<&str>,
    status: ThreadLifecycleStatus,
    created_at: i64,
) -> ThreadSummary {
    ThreadSummary {
        id: id.to_owned(),
        cwd: format!("/repo/{id}"),
        title: title.map(str::to_owned),
        status,
        created_at,
        updated_at: created_at + 1,
        last_sequence: 1,
    }
}

#[test]
fn projection_sorts_filters_and_marks_active_running_threads() {
    let mut state = AiState::default();
    state.threads.insert(
        "older".to_owned(),
        thread("older", Some("Older"), ThreadLifecycleStatus::Idle, 10),
    );
    state.threads.insert(
        "newer".to_owned(),
        thread(
            "newer",
            Some("  Newer  "),
            ThreadLifecycleStatus::Active,
            20,
        ),
    );
    state.threads.insert(
        "archived".to_owned(),
        thread(
            "archived",
            Some("Archived"),
            ThreadLifecycleStatus::Archived,
            30,
        ),
    );
    state.turns.insert(
        "turn".to_owned(),
        TurnSummary {
            id: "turn".to_owned(),
            thread_id: "newer".to_owned(),
            collaboration_mode: None,
            status: TurnStatus::InProgress,
            last_sequence: 2,
        },
    );

    let projection = AiThreadCatalogProjection::from_state(&state, Some("newer"));

    assert_eq!(projection.thread_count, 2);
    assert_eq!(projection.running_thread_count, 1);
    assert_eq!(projection.active_thread_id, "newer");
    assert_eq!(projection.active_thread_title, "Newer");
    assert_eq!(projection.active_thread_cwd, "/repo/newer");
    assert_eq!(projection.items.len(), 2);
    assert_eq!(projection.items[0].thread_id, "newer");
    assert_eq!(projection.items[0].title, "Newer");
    assert!(projection.items[0].active);
    assert!(projection.items[0].running);
    assert_eq!(projection.items[0].workspace_label, "newer");
    assert_eq!(projection.items[1].thread_id, "older");
    assert!(!projection.items[1].active);
}

#[test]
fn projection_bounds_visible_items_but_preserves_total_count() {
    let mut state = AiState::default();
    for index in 0..240 {
        let id = format!("thread-{index:03}");
        state.threads.insert(
            id.clone(),
            thread(
                id.as_str(),
                if index == 239 {
                    Some("  ")
                } else {
                    Some("Thread")
                },
                ThreadLifecycleStatus::Idle,
                index,
            ),
        );
    }

    let projection = AiThreadCatalogProjection::from_state(&state, None);

    assert_eq!(projection.thread_count, 240);
    assert_eq!(projection.items.len(), 200);
    assert_eq!(projection.items[0].thread_id, "thread-239");
    assert_eq!(projection.items[0].title, "Untitled thread");
}

#[test]
fn projection_keeps_an_old_active_thread_inside_the_bounded_catalog() {
    let mut state = AiState::default();
    for index in 0..240 {
        let id = format!("thread-{index:03}");
        state.threads.insert(
            id.clone(),
            thread(
                id.as_str(),
                Some("Thread"),
                ThreadLifecycleStatus::Idle,
                index,
            ),
        );
    }

    let projection = AiThreadCatalogProjection::from_state(&state, Some("thread-000"));

    assert_eq!(projection.items.len(), 200);
    assert!(
        projection
            .items
            .iter()
            .any(|thread| thread.thread_id == "thread-000" && thread.active)
    );
}

#[test]
fn projection_keeps_old_bookmarks_first_inside_the_bounded_catalog() {
    let mut state = AiState::default();
    for index in 0..240 {
        let id = format!("thread-{index:03}");
        state.threads.insert(
            id.clone(),
            thread(
                id.as_str(),
                Some("Thread"),
                ThreadLifecycleStatus::Idle,
                index,
            ),
        );
    }
    let bookmarks = BTreeSet::from(["thread-000".to_owned(), "thread-001".to_owned()]);

    let projection = AiThreadCatalogProjection::from_state_with_bookmarks(&state, None, &bookmarks);

    assert_eq!(projection.items.len(), 200);
    assert_eq!(projection.items[0].thread_id, "thread-001");
    assert_eq!(projection.items[1].thread_id, "thread-000");
    assert!(projection.items[0].bookmarked);
    assert!(projection.items[1].bookmarked);
    assert!(!projection.items[2].bookmarked);
}

#[test]
fn projection_does_not_publish_an_archived_active_thread() {
    let mut state = AiState::default();
    state.threads.insert(
        "archived".to_owned(),
        thread(
            "archived",
            Some("Archived"),
            ThreadLifecycleStatus::Archived,
            30,
        ),
    );

    let projection = AiThreadCatalogProjection::from_state(&state, Some("archived"));

    assert!(projection.active_thread_id.is_empty());
    assert!(projection.active_thread_title.is_empty());
    assert!(projection.active_thread_cwd.is_empty());
    assert!(projection.items.is_empty());
}

#[test]
fn thread_model_skips_identical_streaming_resets() {
    let model = AiThreadListModel::default_with_attached_qobject();
    let mut model = model.borrow_mut();
    let item = AiThreadItem {
        thread_id: "thread".to_owned(),
        title: "Thread".to_owned(),
        ..AiThreadItem::default()
    };

    assert!(model.replace_if_changed(vec![item.clone()]));
    assert_eq!(model.len(), 1);
    assert!(model.contains_thread_id("thread"));
    assert!(!model.contains_thread_id("outside-catalog"));
    assert!(!model.replace_if_changed(vec![item]));
    assert_eq!(model.len(), 1);
}

#[test]
fn thread_model_applies_bookmarks_and_reorders_visible_rows() {
    let model = AiThreadListModel::default_with_attached_qobject();
    let mut model = model.borrow_mut();
    model.replace(vec![
        AiThreadItem {
            thread_id: "newer".to_owned(),
            title: "Newer".to_owned(),
            created_at: 20,
            ..AiThreadItem::default()
        },
        AiThreadItem {
            thread_id: "older".to_owned(),
            title: "Older".to_owned(),
            created_at: 10,
            ..AiThreadItem::default()
        },
    ]);

    assert!(model.apply_bookmarks(&BTreeSet::from(["older".to_owned()])));
    assert_eq!(model.get(0).expect("bookmarked row").thread_id, "older");
    assert!(model.get(0).expect("bookmarked row").bookmarked);
    assert!(!model.get(1).expect("unbookmarked row").bookmarked);
    assert!(!model.apply_bookmarks(&BTreeSet::from(["older".to_owned()])));

    assert!(model.apply_bookmarks(&BTreeSet::new()));
    assert_eq!(model.get(0).expect("newest row").thread_id, "newer");
    assert!(!model.get(1).expect("restored row").bookmarked);
    assert!(!model.apply_bookmarks(&BTreeSet::new()));
}

#[test]
fn projection_marks_threads_that_need_user_attention() {
    let mut state = AiState::default();
    state.threads.insert(
        "thread".to_owned(),
        thread("thread", Some("Thread"), ThreadLifecycleStatus::Active, 1),
    );
    let mut projection = AiThreadCatalogProjection::from_state(&state, Some("thread"));

    projection.mark_attention(&BTreeSet::from(["thread".to_owned()]));

    assert!(projection.items[0].attention);
}
