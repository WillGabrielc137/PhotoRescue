. (Join-Path $PSScriptRoot 'common.ps1')

$root = Get-PhotoRescueRoot
Initialize-PhotoRescueBuildEnvironment

Push-Location $root
try {
    Write-Host 'Gerando executável e instaladores Windows do PhotoRescue...'
    Invoke-PhotoRescueCommand -Command 'npm' -Arguments @(
        '--workspace',
        '@photorescue/desktop',
        'run',
        'tauri:build'
    )

    $executable = Join-Path $root 'target\release\PhotoRescue.exe'
    $msi = Get-ChildItem -LiteralPath (Join-Path $root 'target\release\bundle\msi') `
        -Filter 'PhotoRescue_*.msi' -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    $nsis = Get-ChildItem -LiteralPath (Join-Path $root 'target\release\bundle\nsis') `
        -Filter 'PhotoRescue_*-setup.exe' -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    $artifacts = @(
        $executable,
        $(if ($null -ne $msi) { $msi.FullName } else { $null }),
        $(if ($null -ne $nsis) { $nsis.FullName } else { $null })
    )

    $missing = @($artifacts | Where-Object {
        [string]::IsNullOrWhiteSpace($_) -or -not (Test-Path -LiteralPath $_)
    })
    if ($missing.Count -gt 0) {
        throw "Artefatos esperados não foram criados: $($missing -join ', ')"
    }

    Write-Host ''
    Write-Host 'Arquivos para distribuição:'
    $artifacts | ForEach-Object { Write-Host " - $_" }
}
finally {
    Pop-Location
}
