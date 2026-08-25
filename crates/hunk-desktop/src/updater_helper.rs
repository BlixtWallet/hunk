use std::ffi::{OsStr, OsString};
use std::path::Path;
#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;
use std::process::Command;
#[cfg(not(target_os = "windows"))]
use std::time::Duration;

#[cfg(not(target_os = "windows"))]
use anyhow::anyhow;
use anyhow::{Context, Result, bail};
#[cfg(not(target_os = "windows"))]
use hunk_updater::AssetFormat;
use hunk_updater::{StagedUpdate, UpdateInstallTarget};

const APPLY_STAGED_UPDATE_ARG: &str = "--apply-staged-update";
#[cfg(not(target_os = "windows"))]
const WAIT_PID_ARG: &str = "--wait-pid";
#[cfg(not(target_os = "windows"))]
const PACKAGE_ARG: &str = "--package";
#[cfg(not(target_os = "windows"))]
const FORMAT_ARG: &str = "--asset-format";
#[cfg(not(target_os = "windows"))]
const UPDATE_HELPER_WAIT_TIMEOUT: Duration = Duration::from_secs(90);

pub(crate) fn maybe_handle_updater_helper_mode() -> Result<bool> {
    let mut args = std::env::args_os();
    let _ = args.next();
    match args.next() {
        Some(flag) if flag == OsStr::new(APPLY_STAGED_UPDATE_ARG) => {
            handle_apply_staged_update_helper(args)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub(crate) fn spawn_staged_update_apply(staged_update: &StagedUpdate) -> Result<()> {
    let current_executable =
        std::env::current_exe().context("resolve current Hunk executable for updater install")?;
    let install_target = hunk_updater::detect_install_target(current_executable.as_path())?;
    let current_pid = std::process::id();

    match install_target {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        UpdateInstallTarget::MacOsApp { .. } | UpdateInstallTarget::LinuxBundle { .. } => {
            Command::new(&current_executable)
                .arg(APPLY_STAGED_UPDATE_ARG)
                .arg(WAIT_PID_ARG)
                .arg(current_pid.to_string())
                .arg(PACKAGE_ARG)
                .arg(staged_update.package_path.as_os_str())
                .arg(FORMAT_ARG)
                .arg(staged_update.asset.format.as_str())
                .spawn()
                .with_context(|| {
                    format!("spawn updater helper from {}", current_executable.display())
                })?;
            Ok(())
        }
        #[cfg(target_os = "windows")]
        UpdateInstallTarget::WindowsMsi { current_executable } => {
            spawn_windows_update_script(current_pid, current_executable.as_path(), staged_update)
        }
        #[allow(unreachable_patterns)]
        other => bail!(
            "updater apply helper is not supported for install target {:?} on this platform",
            other
        ),
    }
}

fn handle_apply_staged_update_helper(args: impl Iterator<Item = OsString>) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let _ = args;
        bail!("updater helper mode is not supported on Windows")
    }

    #[cfg(not(target_os = "windows"))]
    {
        let (wait_pid, package_path, asset_format) = parse_helper_arguments(args)?;
        hunk_updater::wait_for_process_to_exit(wait_pid, UPDATE_HELPER_WAIT_TIMEOUT)?;
        let current_executable =
            std::env::current_exe().context("resolve updater helper executable path")?;
        let applied_update = hunk_updater::apply_staged_update_from_current_executable(
            current_executable.as_path(),
            package_path.as_path(),
            asset_format,
        )?;
        launch_updated_app(applied_update.relaunch_executable.as_path())
    }
}

#[cfg(not(target_os = "windows"))]
fn parse_helper_arguments(
    args: impl Iterator<Item = OsString>,
) -> Result<(u32, PathBuf, AssetFormat)> {
    let mut wait_pid = None;
    let mut package_path = None;
    let mut asset_format = None;
    let mut pending_flag: Option<String> = None;

    for arg in args {
        if let Some(flag) = pending_flag.take() {
            match flag.as_str() {
                WAIT_PID_ARG => {
                    let value = arg
                        .to_str()
                        .ok_or_else(|| anyhow!("wait pid must be valid utf-8"))?;
                    wait_pid = Some(
                        value
                            .parse::<u32>()
                            .with_context(|| format!("invalid wait pid `{value}`"))?,
                    );
                }
                PACKAGE_ARG => package_path = Some(PathBuf::from(arg)),
                FORMAT_ARG => {
                    let value = arg
                        .to_str()
                        .ok_or_else(|| anyhow!("asset format must be valid utf-8"))?;
                    asset_format = Some(value.parse::<AssetFormat>()?);
                }
                _ => bail!("unsupported updater helper flag `{flag}`"),
            }
            continue;
        }

        let flag = arg
            .to_str()
            .ok_or_else(|| anyhow!("updater helper flag must be valid utf-8"))?;
        match flag {
            WAIT_PID_ARG | PACKAGE_ARG | FORMAT_ARG => pending_flag = Some(flag.to_owned()),
            other => bail!("unsupported updater helper argument `{other}`"),
        }
    }

    if let Some(flag) = pending_flag {
        bail!("missing value for updater helper flag `{flag}`");
    }
    Ok((
        wait_pid.ok_or_else(|| anyhow!("missing required updater helper flag `{WAIT_PID_ARG}`"))?,
        package_path
            .ok_or_else(|| anyhow!("missing required updater helper flag `{PACKAGE_ARG}`"))?,
        asset_format
            .ok_or_else(|| anyhow!("missing required updater helper flag `{FORMAT_ARG}`"))?,
    ))
}

#[cfg(target_os = "macos")]
fn launch_updated_app(relaunch_executable: &Path) -> Result<()> {
    if let Some(app_path) = relaunch_executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .filter(|candidate| candidate.extension().is_some_and(|value| value == "app"))
    {
        Command::new("open")
            .arg("-n")
            .arg("-a")
            .arg(app_path)
            .spawn()
            .with_context(|| format!("launch updated app bundle {}", app_path.display()))?;
        return Ok(());
    }
    Command::new(relaunch_executable).spawn().with_context(|| {
        format!(
            "launch updated executable {}",
            relaunch_executable.display()
        )
    })?;
    Ok(())
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn launch_updated_app(relaunch_executable: &Path) -> Result<()> {
    Command::new(relaunch_executable).spawn().with_context(|| {
        format!(
            "launch updated executable {}",
            relaunch_executable.display()
        )
    })?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn spawn_windows_update_script(
    current_pid: u32,
    current_executable: &Path,
    staged_update: &StagedUpdate,
) -> Result<()> {
    let script_path = staged_update.package_path.with_extension("ps1");
    let script = format!(
        "$waitPid = {current_pid}\n\
$msiPath = {msi_path}\n\
$appPath = {app_path}\n\
while (Get-Process -Id $waitPid -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 200 }}\n\
$process = Start-Process -FilePath 'msiexec.exe' -ArgumentList @('/i', $msiPath, '/passive', '/norestart') -Wait -PassThru\n\
if ($process.ExitCode -ne 0) {{ exit $process.ExitCode }}\n\
Start-Process -FilePath $appPath\n",
        msi_path = powershell_single_quoted(staged_update.package_path.as_path()),
        app_path = powershell_single_quoted(current_executable),
    );
    std::fs::write(script_path.as_path(), script)
        .with_context(|| format!("write staged update script {}", script_path.display()))?;
    Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(script_path.as_os_str())
        .spawn()
        .context("spawn Windows staged updater helper")?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn powershell_single_quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}
