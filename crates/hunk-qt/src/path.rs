use std::path::PathBuf;

pub fn local_path_from_qml_folder_url(value: &str) -> Result<PathBuf, String> {
    if !value.starts_with("file:") {
        return Ok(PathBuf::from(value));
    }

    url::Url::parse(value)
        .map_err(|error| format!("Invalid repository folder URL: {error}"))?
        .to_file_path()
        .map_err(|()| format!("Repository folder URL is not a local path: {value}"))
}
