param(
    [switch]$CreateShortcut,
    [string]$OfficialDirectory = (Join-Path $env:LOCALAPPDATA 'Cap'),
    [string]$Destination
)

$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$binary = Join-Path $repo 'apps\desktop-gpui\target\x86_64-pc-windows-msvc\release\cap-gpui.exe'
$release = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'release.json')) | ConvertFrom-Json
if (-not $Destination) { $Destination = Join-Path $repo ('dist\Cap-' + $release.version) }
$destination = [IO.Path]::GetFullPath($Destination)
if (-not (Test-Path -LiteralPath $binary)) {
    throw 'The Chinese trial has not been built successfully.'
}
if (Test-Path -LiteralPath $destination) {
    throw "The output directory already exists and will not be overwritten: $destination"
}

$official = [IO.Path]::GetFullPath($OfficialDirectory)
$officialVersion = (Get-Item -LiteralPath (Join-Path $official 'Cap.exe')).VersionInfo.ProductVersion
$upstreamVersion = $release.upstreamTag -replace '^cap-v', ''
if ($officialVersion -ne $upstreamVersion) {
    throw "The unmodified companion program must come from official Cap $upstreamVersion, not $officialVersion"
}
if ((Get-Item -LiteralPath $binary).VersionInfo.ProductVersion -ne $release.version) {
    throw 'The compiled app version does not match release.json.'
}

New-Item -ItemType Directory -Path $destination | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $destination 'Cap.exe')
foreach ($folder in @(
    'target\ffmpeg\bin',
    'target\native-deps\dxc',
    'target\native-deps\onnxruntime\lib'
)) {
    Get-ChildItem -LiteralPath (Join-Path $repo $folder) -File -Filter '*.dll' |
        Copy-Item -Destination $destination
}
Copy-Item -LiteralPath (Join-Path $official 'cap-muxer.exe') -Destination $destination
Copy-Item -LiteralPath (Join-Path $repo 'LICENSE') -Destination (Join-Path $destination 'LICENSE-AGPL.txt')
$sourceTools = Join-Path $destination 'source-tools'
New-Item -ItemType Directory -Path $sourceTools | Out-Null
Get-ChildItem -LiteralPath $PSScriptRoot -Force | Copy-Item -Destination $sourceTools -Recurse

$profile = Join-Path $destination 'profile'
New-Item -ItemType Directory -Path (Join-Path $profile 'recordings') -Force | Out-Null
@{
    general_settings = @{
        hasCompletedStartup = $true
        hasCompletedOnboarding = $true
        autoZoomOnClicks = $true
        postStudioRecordingBehaviour = 'openEditor'
    }
    recording_settings = @{
        mode = 'studio'
    }
} | ConvertTo-Json -Depth 5 |
    Set-Content -LiteralPath (Join-Path $profile 'store') -Encoding utf8NoBOM

$revision = $release.upstreamCommit
$description = @"
Cap 中文试用版（非官方）
版本：$($release.version)
日期：$(Get-Date -Format yyyy-MM-dd)

这是基于 Cap $upstreamVersion 的常用界面汉化版本，不是官方发布。
运行本目录中的 Cap.exe 即可，无需卸载或替换官方版。

已汉化：录制、编辑、缩放、导出、设置、截图编辑器、托盘及右键菜单、悬停提示、常用弹窗、自动化及新手导览等。
保留原文：品牌、设备和字体名称、用户内容、文件名、编码格式及命令示例。
未完全覆盖：云端网页、部分系统弹窗、底层库与服务端返回的动态错误。
没有改动录制、渲染、导出、账号和会员权限的实现。
附带一处上游兼容性修复：为缩略图任务启用计时器，避免 Windows 预览任务报错。

并行试用的区别：
- 配置、录屏、截图和日志保存在本目录下的 profile，不读取官方版的账号。
- 默认使用本地录制模式，录完打开本地编辑器。
- 不注册全局快捷键，不接管 cap:// 登录回调，不自动检查更新。
- 不要在此试用版中安装官方更新；官方版请从原程序更新，以免丢失汉化。
- 仅用于本地录制、编辑和导出的汉化试用；云端账号操作请使用官方版。
- 此程序是本机编译的，未携带 Cap 官方数字签名。

来源与许可：
原始代码：https://github.com/CapSoftware/Cap/tree/$revision
原始版本：$($release.upstreamTag)
本机完整修改源码：$repo
界面框架源码：$(Join-Path (Split-Path -Parent $repo) 'Cap-cn-gpui')
修改补丁：source-changes.patch
汉化词表与应用脚本：source-tools（也保存在 $PSScriptRoot）
主程序遵循随附的 AGPLv3 许可，保留原项目版权。
FFmpeg、DXC 和 ONNX Runtime 来自项目官方依赖准备脚本。
未修改的 cap-muxer.exe 来自匹配的官方 Cap $upstreamVersion。

卸载：退出此试用版后删除本目录和“Cap 中文试用版”快捷方式。
注意：profile 中包含试用版录屏，删除前请另行保存需要的内容。
"@
$description | Set-Content -LiteralPath (Join-Path $destination '使用说明.txt') -Encoding utf8NoBOM
$patchPath = Join-Path $destination 'source-changes.patch'
$null = & git -C $repo rev-parse --verify "$($release.upstreamTag)^{commit}" 2>$null
if ($LASTEXITCODE -eq 0) {
    & git -C $repo diff --binary $release.upstreamTag "--output=$patchPath" -- apps/desktop-gpui
    if ($LASTEXITCODE -ne 0) { throw 'Could not export the source changes.' }
} else {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'upstream-changes.patch') -Destination $patchPath
}

if ($CreateShortcut) {
    $shortcutPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Cap 中文试用版.lnk'
    if (Test-Path -LiteralPath $shortcutPath) {
        throw "The existing shortcut will not be overwritten: $shortcutPath"
    }
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = Join-Path $destination 'Cap.exe'
    $shortcut.WorkingDirectory = $destination
    $shortcut.IconLocation = $shortcut.TargetPath
    $shortcut.Description = "Cap $($release.version) 汉化试用版，配置和录屏与官方版隔离"
    $shortcut.Save()
}

Write-Output $destination
