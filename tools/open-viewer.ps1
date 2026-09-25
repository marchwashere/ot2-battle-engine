$ErrorActionPreference = 'Stop'
$viewerRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$viewerExe = Join-Path $viewerRoot 'target\release\ot2-gui.exe'
try {
    if (-not (Test-Path -LiteralPath $viewerExe)) {
        Push-Location -LiteralPath $viewerRoot
        try {
            & cargo build --offline --release --bin ot2-gui
            if ($LASTEXITCODE -ne 0) { throw 'Viewer build failed. See the compiler output above.' }
        } finally { Pop-Location }
    }
    Start-Process -FilePath $viewerExe -WorkingDirectory $viewerRoot -WindowStyle Hidden
} catch {
    Write-Host $_ -ForegroundColor Red
    exit 1
}
