Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-PhotoRescueRoot {
    return (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
}

function Initialize-PhotoRescueBuildEnvironment {
    $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if (Test-Path -LiteralPath $cargoBin) {
        $pathEntries = $env:PATH -split ';'
        if ($cargoBin -notin $pathEntries) {
            $env:PATH = "$cargoBin;$env:PATH"
        }
    }

    foreach ($commandName in @('node', 'npm', 'cargo', 'rustc')) {
        if (-not (Get-Command $commandName -ErrorAction SilentlyContinue)) {
            throw "Dependência ausente: $commandName. Consulte o README.md."
        }
    }
}

function Invoke-PhotoRescueCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter()]
        [string[]]$Arguments = @()
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "O comando '$Command $($Arguments -join ' ')' terminou com código $LASTEXITCODE."
    }
}

