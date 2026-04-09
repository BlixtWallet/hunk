pub(crate) fn is_missing_repository_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        let message = cause.to_string().to_ascii_lowercase();
        message.contains("failed to discover git repository")
            || message.contains("could not find repository")
            || message.contains("not a git repository")
    })
}
