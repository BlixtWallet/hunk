use hunk_browser::{BrowserFrameMetadata, BrowserTabId, BrowserTabSummary};
use hunk_desktop::{
    BrowserTabItem, BrowserTabListModel, browser_tab_projection_changed,
    project_browser_tab_sources, project_browser_tabs,
};
use qtbridge::{QListModel, QObjectHolder};

fn browser_tab(url: Option<&str>, title: Option<&str>, loading: bool) -> BrowserTabSummary {
    BrowserTabSummary {
        tab_id: BrowserTabId::new("tab-1"),
        url: url.map(ToOwned::to_owned),
        title: title.map(ToOwned::to_owned),
        loading,
        load_error: None,
        can_go_back: false,
        can_go_forward: false,
        snapshot_epoch: 0,
        latest_frame: None,
    }
}

#[test]
fn browser_tab_projection_preserves_state_and_uses_bounded_fallback_titles() {
    let long_title = "界".repeat(100);
    let tabs = [
        browser_tab(Some("about:blank"), None, false),
        browser_tab(
            Some("https://doc.qt.io/qt-6/qtquick-index.html"),
            Some("  "),
            true,
        ),
        browser_tab(Some("local-page"), None, false),
        browser_tab(Some("https://example.com"), Some("Example"), false),
        browser_tab(Some("https://large.example.com"), Some(&long_title), false),
    ];

    let projection = project_browser_tabs(&tabs);

    assert_eq!(projection[0].title, "New tab");
    assert_eq!(projection[1].title, "doc.qt.io");
    assert!(projection[1].loading);
    assert_eq!(projection[2].title, "local-page");
    assert_eq!(projection[3].title, "Example");
    assert!(projection[4].title.len() <= 240);
    assert!(projection[4].title.ends_with('…'));
}

#[test]
fn browser_tab_model_replaces_rows_without_retaining_stale_tabs() {
    let model = BrowserTabListModel::default_with_attached_qobject();
    let mut model = model.borrow_mut();
    model.replace_or_patch(vec![BrowserTabItem {
        tab_id: "tab-1".to_owned(),
        title: "First".to_owned(),
        loading: false,
    }]);
    model.replace_or_patch(vec![BrowserTabItem {
        tab_id: "tab-2".to_owned(),
        title: "Second".to_owned(),
        loading: true,
    }]);

    assert_eq!(model.len(), 1);
    let tab = model.get(0).expect("replacement browser tab");
    assert_eq!(tab.tab_id, "tab-2");
    assert_eq!(tab.title, "Second");
    assert!(tab.loading);
}

#[test]
fn browser_tab_projection_cache_ignores_frame_only_updates() {
    let mut tabs = vec![browser_tab(
        Some("https://example.com"),
        Some("Example"),
        false,
    )];
    let sources = project_browser_tab_sources(&tabs);

    tabs[0].latest_frame = Some(BrowserFrameMetadata {
        width: 1280,
        height: 720,
        frame_epoch: 42,
    });
    assert!(!browser_tab_projection_changed(&sources, &tabs));

    tabs[0].title = Some("Updated".to_owned());
    assert!(browser_tab_projection_changed(&sources, &tabs));
}
