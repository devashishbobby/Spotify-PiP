param(
    [string]$ExecutablePath = (Join-Path $PSScriptRoot '..\src-tauri\target\release\spotify-pip.exe'),
    [string]$IconPath = (Join-Path $PSScriptRoot '..\src-tauri\icons\icon.ico')
)

$ErrorActionPreference = 'Stop'
$executable = (Resolve-Path -LiteralPath $ExecutablePath).Path
$desktop = [Environment]::GetFolderPath('DesktopDirectory')
$shortcutPath = Join-Path $desktop 'SpotifyPiP.lnk'
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $executable
$shortcut.WorkingDirectory = Split-Path -Parent $executable
if (Test-Path -LiteralPath $IconPath) {
    $shortcut.IconLocation = "$((Resolve-Path -LiteralPath $IconPath).Path),0"
} else {
    $shortcut.IconLocation = "$executable,0"
}
$shortcut.Description = 'Open SpotifyPiP music player'
$shortcut.Save()
Write-Output "Created $shortcutPath -> $executable"
