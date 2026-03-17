param(
    [string]$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

function Convert-BashPathToWindowsPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if ($Path -match '^/mnt/([A-Za-z])/(.*)$') {
        $drive = $Matches[1].ToUpperInvariant()
        $rest = $Matches[2].Replace('/', '\')
        return "{0}:\{1}" -f $drive, $rest
    }

    if ($Path -match '^/([A-Za-z])/(.*)$') {
        $drive = $Matches[1].ToUpperInvariant()
        $rest = $Matches[2].Replace('/', '\')
        return "{0}:\{1}" -f $drive, $rest
    }

    if ($Path -match '^[A-Za-z]:/') {
        return $Path.Replace('/', '\')
    }

    return $Path
}

Push-Location $RootDir
try {
    # Try the existing bash-based resolver first (WSL / Git Bash / MSYS)
    try {
        $bashOutput = & bash ./scripts/resolve_cargo_target_dir.sh 2>$null
        if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($bashOutput)) {
            $targetDir = $bashOutput.Trim()
        }
    } catch {
        # ignore and try native fallback
    }

    # Native PowerShell fallback if bash/WSL is not available or failed
    if ([string]::IsNullOrWhiteSpace($targetDir)) {
        if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
            $targetDir = $env:CARGO_TARGET_DIR
        } else {
            try {
                $gitCommon = git -C $RootDir rev-parse --path-format=absolute --git-common-dir 2>$null
                if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($gitCommon)) {
                    $sharedRoot = [System.IO.Path]::GetFullPath((Join-Path $gitCommon ".."))
                    $targetDir = Join-Path $sharedRoot "target-shared"
                }
            } catch {
                # git not available or not a repo; fall back to default
            }

            if ([string]::IsNullOrWhiteSpace($targetDir)) {
                $targetDir = Join-Path $RootDir "target"
            }
        }
    }
} finally {
    Pop-Location
}

if ([string]::IsNullOrWhiteSpace($targetDir)) {
    throw "Failed to resolve CARGO_TARGET_DIR via scripts/resolve_cargo_target_dir.sh or native fallback"
}

Write-Output ([System.IO.Path]::GetFullPath((Convert-BashPathToWindowsPath -Path $targetDir)))
