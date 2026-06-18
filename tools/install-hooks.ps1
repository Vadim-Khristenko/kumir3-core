#!/usr/bin/env pwsh
# =============================================================================
#         KITE :: HOOK INSTALLER (PowerShell)
# =============================================================================
# Points git at the version-controlled .githooks/ directory.
# Run once after cloning:  pwsh tools/install-hooks.ps1
# (The hooks themselves are bash scripts; Git for Windows ships bash.)
# =============================================================================

$ErrorActionPreference = 'Stop'

Set-Location (Join-Path $PSScriptRoot '..')

git config core.hooksPath .githooks

Write-Host '✔ KITE git hooks installed (core.hooksPath -> .githooks).'
