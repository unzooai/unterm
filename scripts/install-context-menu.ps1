# Unterm 右键菜单注册脚本
# 用法: 右键以管理员身份运行，或在 PowerShell 中执行: .\install-context-menu.ps1
# 自动检测 unterm-app.exe 位置

param(
    [string]$ExePath
)

# 自动查找 exe
if (-not $ExePath) {
    $candidates = @(
        "$PSScriptRoot\..\target\release\unterm-app.exe",
        "$PSScriptRoot\..\target\debug\unterm-app.exe",
        "$env:ProgramFiles\Unterm\unterm-app.exe",
        "$env:LOCALAPPDATA\Unterm\unterm-app.exe"
    )
    foreach ($c in $candidates) {
        if (Test-Path $c) {
            $ExePath = (Resolve-Path $c).Path
            break
        }
    }
}

if (-not $ExePath -or -not (Test-Path $ExePath)) {
    Write-Host "未找到 unterm-app.exe，请指定路径: .\install-context-menu.ps1 -ExePath 'C:\path\to\unterm-app.exe'" -ForegroundColor Red
    exit 1
}

$ExePath = $ExePath.Replace('/', '\')
Write-Host "使用路径: $ExePath" -ForegroundColor Green

# 注册右键菜单 — 文件夹背景
$bgKey = "HKCU:\Software\Classes\Directory\Background\shell\Unterm"
New-Item -Path $bgKey -Force | Out-Null
Set-ItemProperty -Path $bgKey -Name "(Default)" -Value "在 Unterm 中打开"
Set-ItemProperty -Path $bgKey -Name "Icon" -Value "`"$ExePath`""
New-Item -Path "$bgKey\command" -Force | Out-Null
Set-ItemProperty -Path "$bgKey\command" -Name "(Default)" -Value "`"$ExePath`" `"%V`""

# 注册右键菜单 — 文件夹
$dirKey = "HKCU:\Software\Classes\Directory\shell\Unterm"
New-Item -Path $dirKey -Force | Out-Null
Set-ItemProperty -Path $dirKey -Name "(Default)" -Value "在 Unterm 中打开"
Set-ItemProperty -Path $dirKey -Name "Icon" -Value "`"$ExePath`""
New-Item -Path "$dirKey\command" -Force | Out-Null
Set-ItemProperty -Path "$dirKey\command" -Name "(Default)" -Value "`"$ExePath`" `"%V`""

Write-Host "右键菜单已注册！在文件夹中右键即可看到「在 Unterm 中打开」" -ForegroundColor Green
