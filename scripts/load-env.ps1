# Load repo-root .env into the current PowerShell session.
# Usage: . .\scripts\load-env.ps1
param(
    [string]$EnvFile = (Join-Path (Split-Path $PSScriptRoot -Parent) ".env")
)

if (-not (Test-Path $EnvFile)) {
    Write-Error "Missing env file: $EnvFile"
    return
}

Get-Content $EnvFile | ForEach-Object {
    if ($_ -match '^\s*#' -or $_ -match '^\s*$') { return }
    $name, $value = $_ -split '=', 2
    if ($name) {
        Set-Item -Path "Env:$name" -Value $value
    }
}

Write-Host "Loaded environment from $EnvFile"
