. (Join-Path $PSScriptRoot 'common.ps1')

$root = Get-PhotoRescueRoot
Initialize-PhotoRescueBuildEnvironment

Push-Location $root
try {
    Invoke-PhotoRescueCommand -Command 'npm' -Arguments @(
        '--workspace',
        '@photorescue/desktop',
        'run',
        'tauri:dev'
    )
}
finally {
    Pop-Location
}

