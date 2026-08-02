$ErrorActionPreference = "Continue"

function Confirm-Action {
    param([string]$Prompt)
    $answer = Read-Host "$Prompt (y/N)"
    return ($answer -match '^(y|yes)$')
}

$cmd = Get-Command leaf -CommandType Application -ErrorAction SilentlyContinue
if (-not $cmd) {
    [Console]::Error.WriteLine("leaf not found in PATH. Nothing to uninstall.")
    exit 1
}
$bin = $cmd.Source

Write-Host "Uninstalling leaf ($bin)..."

& $bin --config remove

& $bin --auto-complete remove

if ($bin -match '\\scoop\\') {
    Write-Host "Detected Scoop installation."
    if (Confirm-Action "Run 'scoop uninstall leaf-markdown-viewer'?") {
        scoop uninstall leaf-markdown-viewer
    }
    exit 0
}
if ($bin -match '\\\.cargo\\bin\\') {
    Write-Host "Detected Cargo installation."
    if (Confirm-Action "Run 'cargo uninstall leaf-markdown-viewer'?") {
        cargo uninstall leaf-markdown-viewer
    }
    exit 0
}
if ($bin -match '\\pnpm\\') {
    Write-Host "Detected pnpm installation."
    if (Confirm-Action "Run 'pnpm uninstall -g @rivolink/leaf'?") {
        pnpm uninstall -g '@rivolink/leaf'
    }
    exit 0
}
if ($bin -match '\\\.yarn\\|\\Yarn\\') {
    Write-Host "Detected yarn installation."
    if (Confirm-Action "Run 'yarn global remove @rivolink/leaf'?") {
        yarn global remove '@rivolink/leaf'
    }
    exit 0
}
if ($bin -match '\\npm\\leaf\.(cmd|ps1)$') {
    Write-Host "Detected npm installation."
    if (Confirm-Action "Run 'npm uninstall -g @rivolink/leaf'?") {
        npm uninstall -g '@rivolink/leaf'
    }
    exit 0
}

if (-not (Confirm-Action "Remove binary $bin?")) {
    Write-Host "Binary removal cancelled."
    exit 0
}

Remove-Item -Path $bin -Force -ErrorAction SilentlyContinue

$installDir = Join-Path $env:LOCALAPPDATA "Programs\leaf"
if ((Split-Path -Parent $bin) -ieq $installDir -and (Test-Path $installDir)) {
    Remove-Item -Path $installDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "Install directory removed: $installDir"
}

if (Test-Path $bin) {
    [Console]::Error.WriteLine("Failed to remove $bin.")
    [Console]::Error.WriteLine("Close any running leaf process and retry.")
    exit 1
}

Write-Host "leaf binary removed: $bin"
