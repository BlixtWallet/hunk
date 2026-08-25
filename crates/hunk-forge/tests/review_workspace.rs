use std::cell::Cell;

use anyhow::Result;
use hunk_forge::{
    CreateReviewInput, CreateReviewResult, ForgeProvider, ForgeRepoRef, ForgeReviewApi,
    ForgeReviewState, ForgeReviewWorkspace, OpenReviewQuery, OpenReviewSummary,
    find_or_create_review,
};
use hunk_git::branch::ReviewRemote;
use hunk_git::config::ReviewProviderKind;

struct FakeReviewApi {
    existing: Option<OpenReviewSummary>,
    create_calls: Cell<usize>,
}

impl ForgeReviewApi for FakeReviewApi {
    fn find_open_review(&self, _query: &OpenReviewQuery) -> Result<Option<OpenReviewSummary>> {
        Ok(self.existing.clone())
    }

    fn find_branch_review(&self, _query: &OpenReviewQuery) -> Result<Option<OpenReviewSummary>> {
        Ok(self.existing.clone())
    }

    fn create_review(&self, input: &CreateReviewInput) -> Result<CreateReviewResult> {
        self.create_calls.set(self.create_calls.get() + 1);
        Ok(CreateReviewResult {
            review: review(42, input.title.as_str()),
        })
    }
}

#[test]
fn find_or_create_reuses_an_open_review() -> Result<()> {
    let existing = review(17, "Existing review");
    let client = FakeReviewApi {
        existing: Some(existing.clone()),
        create_calls: Cell::new(0),
    };

    let outcome = find_or_create_review(
        &client,
        &workspace(),
        "main",
        "Replacement title",
        None,
        false,
    )?;

    assert!(outcome.existed);
    assert_eq!(outcome.review, existing);
    assert_eq!(client.create_calls.get(), 0);
    Ok(())
}

#[test]
fn find_or_create_creates_only_after_lookup_misses() -> Result<()> {
    let client = FakeReviewApi {
        existing: None,
        create_calls: Cell::new(0),
    };

    let outcome = find_or_create_review(
        &client,
        &workspace(),
        "main",
        "Qt forge controls",
        Some("Migrate forge actions".to_string()),
        true,
    )?;

    assert!(!outcome.existed);
    assert_eq!(outcome.review.number, 42);
    assert_eq!(outcome.review.title, "Qt forge controls");
    assert_eq!(client.create_calls.get(), 1);
    Ok(())
}

#[test]
fn find_or_create_rejects_invalid_review_fields_before_network_work() {
    let client = FakeReviewApi {
        existing: None,
        create_calls: Cell::new(0),
    };

    let error = find_or_create_review(&client, &workspace(), "feature", "", None, false)
        .expect_err("source and base branch must differ");

    assert!(error.to_string().contains("base branch"));
    assert_eq!(client.create_calls.get(), 0);
}

fn workspace() -> ForgeReviewWorkspace {
    let repo = github_repo();
    ForgeReviewWorkspace {
        review_remote: ReviewRemote {
            provider: ReviewProviderKind::GitHub,
            host: "github.com".to_string(),
            authority: "github.com".to_string(),
            repository_path: "smolcars/hunk".to_string(),
            base_url: "https://github.com/smolcars/hunk".to_string(),
        },
        base_repo: repo.clone(),
        head_repo: repo,
        source_branch: "feature".to_string(),
        target_branch: "main".to_string(),
    }
}

fn github_repo() -> ForgeRepoRef {
    ForgeRepoRef {
        provider: ForgeProvider::GitHub,
        host: "github.com".to_string(),
        authority: "github.com".to_string(),
        namespace: "smolcars".to_string(),
        name: "hunk".to_string(),
        path: "smolcars/hunk".to_string(),
        web_base_url: "https://github.com/smolcars/hunk".to_string(),
    }
}

fn review(number: u64, title: &str) -> OpenReviewSummary {
    OpenReviewSummary {
        provider: ForgeProvider::GitHub,
        number,
        title: title.to_string(),
        url: format!("https://github.com/smolcars/hunk/pull/{number}"),
        state: ForgeReviewState::Open,
        draft: false,
        source_branch: "feature".to_string(),
        target_branch: "main".to_string(),
    }
}
