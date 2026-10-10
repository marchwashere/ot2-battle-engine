$ErrorActionPreference = 'Stop'
# Compatibility entry point for existing desktop shortcuts. UI belongs to the
# assistant repository; this battle crate has no viewer target or dependency.
$assistantLauncher = Join-Path $PSScriptRoot '..\..\OT2SpeedrunAssistant\tools\open-viewer.ps1'
if (-not (Test-Path -LiteralPath $assistantLauncher)) { throw 'The sibling OT2SpeedrunAssistant repository is required for the GUI. Battle CLI remains standalone.' }
& $assistantLauncher
