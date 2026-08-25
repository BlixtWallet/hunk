use hunk_desktop::{AiThreadActionReceipt, AiThreadCatalogProjection, AiThreadItem};

fn projection(active_thread_id: &str, thread_ids: &[&str]) -> AiThreadCatalogProjection {
    AiThreadCatalogProjection {
        items: thread_ids
            .iter()
            .map(|thread_id| AiThreadItem {
                thread_id: (*thread_id).to_owned(),
                active: *thread_id == active_thread_id,
                ..AiThreadItem::default()
            })
            .collect(),
        active_thread_id: active_thread_id.to_owned(),
        ..AiThreadCatalogProjection::default()
    }
}

#[test]
fn select_completes_only_when_the_exact_thread_becomes_active() {
    let receipt = AiThreadActionReceipt::select("target".to_owned());

    assert!(!receipt.is_complete(&projection("other", &["target", "other"])));
    assert!(receipt.is_complete(&projection("target", &["target", "other"])));
}

#[test]
fn create_and_fork_wait_for_the_started_thread_snapshot() {
    for mut receipt in [
        AiThreadActionReceipt::create(),
        AiThreadActionReceipt::fork("source".to_owned()),
    ] {
        assert!(!receipt.is_complete(&projection("new", &["source", "new"])));
        assert!(receipt.record_started_thread("new".to_owned()));
        assert!(!receipt.is_complete(&projection("source", &["source", "new"])));
        assert!(receipt.is_complete(&projection("new", &["source", "new"])));
    }
}

#[test]
fn non_creation_receipts_ignore_started_thread_events() {
    let mut select = AiThreadActionReceipt::select("target".to_owned());
    let mut archive = AiThreadActionReceipt::archive("target".to_owned());

    assert!(!select.record_started_thread("unexpected".to_owned()));
    assert!(!archive.record_started_thread("unexpected".to_owned()));
}

#[test]
fn archive_completes_only_after_the_exact_thread_leaves_the_catalog() {
    let receipt = AiThreadActionReceipt::archive("target".to_owned());

    assert!(!receipt.is_complete(&projection("target", &["target", "other"])));
    assert!(!receipt.is_complete(&projection("other", &["target", "other"])));
    assert!(receipt.is_complete(&projection("other", &["other"])));
}
