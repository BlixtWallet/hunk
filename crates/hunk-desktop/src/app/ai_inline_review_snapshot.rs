use std::collections::BTreeMap;

use hunk_git::compare::CompareSnapshot;
use hunk_git::git::{ChangedFile, FileStatus, LineStats};

const SYNTHETIC_TURN_DIFF_PATH_BASENAME: &str = "historical-turn-diff.patch";

#[derive(Debug, Clone, PartialEq, Eq)]
struct TurnDiffPatchSection {
    path: String,
    status: FileStatus,
    patch: String,
    line_stats: LineStats,
}

pub(crate) fn compare_snapshot_from_turn_diff(diff: &str) -> CompareSnapshot {
    let mut files = Vec::<ChangedFile>::new();
    let mut file_line_stats = BTreeMap::<String, LineStats>::new();
    let mut patches_by_path = BTreeMap::<String, String>::new();
    let mut overall_line_stats = LineStats::default();

    for section in split_turn_diff_sections(diff)
        .into_iter()
        .enumerate()
        .filter_map(|(section_ix, section)| {
            parse_turn_diff_patch_section(section.as_str(), section_ix)
        })
    {
        if let Some(existing_patch) = patches_by_path.get_mut(section.path.as_str()) {
            if !existing_patch.ends_with('\n') {
                existing_patch.push('\n');
            }
            existing_patch.push_str(section.patch.as_str());
        } else {
            files.push(ChangedFile {
                path: section.path.clone(),
                status: section.status,
                staged: false,
                unstaged: false,
                untracked: section.status == FileStatus::Added,
            });
            patches_by_path.insert(section.path.clone(), section.patch.clone());
        }

        let entry = file_line_stats.entry(section.path.clone()).or_default();
        entry.added = entry.added.saturating_add(section.line_stats.added);
        entry.removed = entry.removed.saturating_add(section.line_stats.removed);
        overall_line_stats.added = overall_line_stats
            .added
            .saturating_add(section.line_stats.added);
        overall_line_stats.removed = overall_line_stats
            .removed
            .saturating_add(section.line_stats.removed);
    }

    CompareSnapshot {
        files,
        file_line_stats,
        overall_line_stats,
        patches_by_path,
    }
}

fn split_turn_diff_sections(diff: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();
    let mut current_has_hunk_header = false;

    for line in diff.lines() {
        let starts_new_section = line.starts_with("diff --git ")
            || (line.starts_with("--- ") && !current.trim().is_empty() && current_has_hunk_header);
        if starts_new_section && !current.trim().is_empty() {
            sections.push(std::mem::take(&mut current));
            current_has_hunk_header = false;
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
        if line.starts_with("@@") {
            current_has_hunk_header = true;
        }
    }

    if !current.trim().is_empty() {
        sections.push(current);
    }

    sections
}

fn parse_turn_diff_patch_section(section: &str, section_ix: usize) -> Option<TurnDiffPatchSection> {
    let lines = section.lines().collect::<Vec<_>>();
    let (old_path, new_path) = lines
        .first()
        .and_then(|line| diff_git_paths(line))
        .or_else(|| unified_patch_paths(lines.as_slice()))
        .unwrap_or((None, None));

    let rename_from = lines
        .iter()
        .find_map(|line| line.strip_prefix("rename from "))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned);
    let rename_to = lines
        .iter()
        .find_map(|line| line.strip_prefix("rename to "))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned);
    let is_added = lines.iter().any(|line| line.starts_with("new file mode "));
    let is_deleted = lines
        .iter()
        .any(|line| line.starts_with("deleted file mode "));
    let is_renamed = rename_from.is_some() || rename_to.is_some();
    let path = rename_to
        .clone()
        .or_else(|| pick_patch_display_path(old_path.as_deref(), new_path.as_deref()))
        .or_else(|| {
            lines
                .iter()
                .any(|line| line.starts_with("@@"))
                .then(|| synthetic_turn_diff_path(section_ix))
        })
        .filter(|path| !path.is_empty())?;
    let status = if is_added || (old_path.is_none() && new_path.is_some()) {
        FileStatus::Added
    } else if is_deleted || (old_path.is_some() && new_path.is_none()) {
        FileStatus::Deleted
    } else if is_renamed {
        FileStatus::Renamed
    } else {
        FileStatus::Modified
    };

    Some(TurnDiffPatchSection {
        path,
        status,
        patch: section.to_string(),
        line_stats: unified_patch_line_stats(section),
    })
}

fn unified_patch_paths(lines: &[&str]) -> Option<(Option<String>, Option<String>)> {
    let old_path = lines
        .iter()
        .find_map(|line| line.strip_prefix("--- "))
        .and_then(normalize_patch_path);
    let new_path = lines
        .iter()
        .find_map(|line| line.strip_prefix("+++ "))
        .and_then(normalize_patch_path);
    (old_path.is_some() || new_path.is_some()).then_some((old_path, new_path))
}

fn diff_git_paths(line: &str) -> Option<(Option<String>, Option<String>)> {
    let mut parts = line.split_whitespace();
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("diff"), Some("--git"), Some(old_path), Some(new_path)) => Some((
            normalize_patch_path(old_path),
            normalize_patch_path(new_path),
        )),
        _ => None,
    }
}

fn normalize_patch_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() || path == "/dev/null" {
        return None;
    }
    Some(
        path.strip_prefix("a/")
            .or_else(|| path.strip_prefix("b/"))
            .unwrap_or(path)
            .to_string(),
    )
}

fn pick_patch_display_path(old_path: Option<&str>, new_path: Option<&str>) -> Option<String> {
    new_path
        .filter(|path| *path != "/dev/null")
        .map(ToOwned::to_owned)
        .or_else(|| old_path.map(ToOwned::to_owned))
}

fn synthetic_turn_diff_path(section_ix: usize) -> String {
    if section_ix == 0 {
        return SYNTHETIC_TURN_DIFF_PATH_BASENAME.to_string();
    }

    format!(
        "historical-turn-diff-{}.patch",
        section_ix.saturating_add(1)
    )
}

fn unified_patch_line_stats(diff: &str) -> LineStats {
    let mut added = 0u64;
    let mut removed = 0u64;

    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            added = added.saturating_add(1);
        } else if line.starts_with('-') {
            removed = removed.saturating_add(1);
        }
    }

    LineStats { added, removed }
}

#[cfg(test)]
mod tests {
    use super::compare_snapshot_from_turn_diff;
    use hunk_git::git::FileStatus;

    #[test]
    fn compare_snapshot_from_turn_diff_splits_files_and_counts_lines() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1,2 @@
-old
+new
+extra
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -5 +5 @@
-before
+after
";

        let snapshot = compare_snapshot_from_turn_diff(diff);

        assert_eq!(snapshot.files.len(), 2);
        assert_eq!(snapshot.files[0].path, "src/lib.rs");
        assert_eq!(snapshot.files[0].status, FileStatus::Modified);
        assert_eq!(snapshot.file_line_stats["src/lib.rs"].added, 2);
        assert_eq!(snapshot.file_line_stats["src/lib.rs"].removed, 1);
        assert_eq!(snapshot.overall_line_stats.added, 3);
        assert_eq!(snapshot.overall_line_stats.removed, 2);
    }

    #[test]
    fn compare_snapshot_from_turn_diff_handles_add_delete_and_rename() {
        let diff = "\
diff --git a/src/new.rs b/src/new.rs
new file mode 100644
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1 @@
+hello
diff --git a/src/old.rs b/src/old.rs
deleted file mode 100644
--- a/src/old.rs
+++ /dev/null
@@ -1 +0,0 @@
-goodbye
diff --git a/src/from.rs b/src/to.rs
rename from src/from.rs
rename to src/to.rs
--- a/src/from.rs
+++ b/src/to.rs
@@ -1 +1 @@
-before
+after
";

        let snapshot = compare_snapshot_from_turn_diff(diff);

        assert_eq!(snapshot.files.len(), 3);
        assert_eq!(snapshot.files[0].status, FileStatus::Added);
        assert_eq!(snapshot.files[1].status, FileStatus::Deleted);
        assert_eq!(snapshot.files[2].status, FileStatus::Renamed);
        assert_eq!(snapshot.files[2].path, "src/to.rs");
    }

    #[test]
    fn compare_snapshot_from_turn_diff_merges_duplicate_paths() {
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-one
+two
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -3 +3 @@
-three
+four
";

        let snapshot = compare_snapshot_from_turn_diff(diff);

        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.file_line_stats["src/lib.rs"].added, 2);
        assert_eq!(snapshot.file_line_stats["src/lib.rs"].removed, 2);
        assert!(snapshot.patches_by_path["src/lib.rs"].contains("@@ -1 +1 @@"));
        assert!(snapshot.patches_by_path["src/lib.rs"].contains("@@ -3 +3 @@"));
    }

    #[test]
    fn compare_snapshot_from_headerless_turn_diff_recovers_paths() {
        let diff = "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-old
+new
--- a/src/main.rs
+++ b/src/main.rs
@@ -5 +5 @@
-before
+after
";

        let snapshot = compare_snapshot_from_turn_diff(diff);

        assert_eq!(snapshot.files.len(), 2);
        assert_eq!(snapshot.files[0].path, "src/lib.rs");
        assert_eq!(snapshot.files[1].path, "src/main.rs");
        assert_eq!(snapshot.overall_line_stats.added, 2);
        assert_eq!(snapshot.overall_line_stats.removed, 2);
    }

    #[test]
    fn compare_snapshot_from_pathless_turn_diff_uses_synthetic_file() {
        let diff = "\
@@ -1 +1 @@
-old
+new
";

        let snapshot = compare_snapshot_from_turn_diff(diff);

        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.files[0].path, "historical-turn-diff.patch");
        assert_eq!(snapshot.files[0].status, FileStatus::Modified);
        assert_eq!(
            snapshot.file_line_stats["historical-turn-diff.patch"].added,
            1
        );
        assert_eq!(
            snapshot.file_line_stats["historical-turn-diff.patch"].removed,
            1
        );
        assert_eq!(
            snapshot.patches_by_path["historical-turn-diff.patch"],
            diff.trim_end()
        );
    }
}
