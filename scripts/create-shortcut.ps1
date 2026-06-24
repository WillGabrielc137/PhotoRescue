param(
    [Parameter()]
    [string]$TargetPath,

    [Parameter()]
    [string]$ShortcutName = 'PhotoRescue'
)

. (Join-Path $PSScriptRoot 'common.ps1')

$root = Get-PhotoRescueRoot

if ([string]::IsNullOrWhiteSpace($TargetPath)) {
    $candidates = @(
        (Join-Path $root 'target\release\PhotoRescue.exe'),
        (Join-Path $env:LOCALAPPDATA 'PhotoRescue\PhotoRescue.exe'),
        (Join-Path $env:ProgramFiles 'PhotoRescue\PhotoRescue.exe')
    )

    $TargetPath = $candidates |
        Where-Object { Test-Path -LiteralPath $_ } |
        Select-Object -First 1
}

if ([string]::IsNullOrWhiteSpace($TargetPath)) {
    throw 'Nenhum executável foi encontrado. Execute npm run build ou instale o PhotoRescue primeiro.'
}

$resolvedTarget = (Resolve-Path -LiteralPath $TargetPath).Path
if ([System.IO.Path]::GetExtension($resolvedTarget) -ne '.exe') {
    throw "O destino do atalho precisa ser um executável .exe: $resolvedTarget"
}

$desktop = [Environment]::GetFolderPath([Environment+SpecialFolder]::DesktopDirectory)
if ([string]::IsNullOrWhiteSpace($desktop)) {
    throw 'Não foi possível identificar a Área de Trabalho do usuário.'
}

$shortcutPath = Join-Path $desktop "$ShortcutName.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $resolvedTarget
$shortcut.WorkingDirectory = Split-Path -Parent $resolvedTarget
$shortcut.IconLocation = "$resolvedTarget,0"
$shortcut.Description = 'PhotoRescue - recuperação segura de imagens'
$shortcut.WindowStyle = 1
$shortcut.Save()

if (-not (Test-Path -LiteralPath $shortcutPath)) {
    throw "O atalho não foi criado: $shortcutPath"
}

Write-Host "Atalho criado em: $shortcutPath"
Write-Host "Destino: $resolvedTarget"

