use std::path::{Path, PathBuf};

use crate::session::BrowserError;

pub(super) fn stage_for_bare_run(framework_dir: &Path) -> Result<(), BrowserError> {
    let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|current_exe| current_exe.parent().map(PathBuf::from))
    else {
        return Ok(());
    };
    if exe_dir.file_name().is_some_and(|name| name == "MacOS")
        && exe_dir.parent().is_some_and(|contents_dir| {
            contents_dir
                .file_name()
                .is_some_and(|name| name == "Contents")
        })
    {
        return Ok(());
    }

    let libraries_dir = framework_dir.join("Libraries");
    for sidecar in [
        "libEGL.dylib",
        "libGLESv2.dylib",
        "libvk_swiftshader.dylib",
        "vk_swiftshader_icd.json",
    ] {
        let source = libraries_dir.join(sidecar);
        if !source.is_file() {
            return Err(backend_error(format!(
                "Chromium Embedded Framework sidecar is missing {}",
                source.display()
            )));
        }

        let dest = exe_dir.join(sidecar);
        if dest.is_file() {
            continue;
        }
        std::fs::copy(&source, &dest).map_err(|error| {
            backend_error(format!(
                "failed to stage Chromium Embedded Framework sidecar {} to {}: {error}",
                source.display(),
                dest.display()
            ))
        })?;
    }

    Ok(())
}

fn backend_error(message: impl Into<String>) -> BrowserError {
    BrowserError::BackendUnavailable(message.into())
}
