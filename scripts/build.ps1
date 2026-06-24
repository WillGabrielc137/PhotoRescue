. (Join-Path $PSScriptRoot 'common.ps1')

$root = Get-PhotoRescueRoot
Initialize-PhotoRescueBuildEnvironment

Push-Location $root
try {
    Write-Host 'Gerando executável de produção do PhotoRescue...'
    Invoke-PhotoRescueCommand -Command 'npm' -Arguments @(
        '--workspace',
        '@photorescue/desktop',
        'run',
        'tauri:build',
        '--',
        '--no-bundle'
    )

    $executable = Join-Path $root 'target\release\PhotoRescue.exe'
    if (-not (Test-Path -LiteralPath $executable)) {
        throw "O executável esperado não foi criado: $executable"
    }

    Write-Host ''
    Write-Host "Executável criado em: $executable"
}
finally {
    Pop-Location
}

