param(
    [Parameter(Mandatory)][string]$PackageDirectory,
    [string]$CompilerPath = 'E:\Agentspace\dev-tools\innosetup-6.4.3\ISCC.exe',
    [string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$version = '0.6.1-cn.2-dual-preview.4'
$tag = "v$version"
$package = [IO.Path]::GetFullPath($PackageDirectory)
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repo 'dist\preview-4' }
$output = [IO.Path]::GetFullPath($OutputDirectory)
$release = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'release.json') -Raw | ConvertFrom-Json
if ((Get-Item -LiteralPath (Join-Path $package 'Cap.exe')).VersionInfo.ProductVersion -ne $release.version) {
    throw '主程序基础版本不匹配。'
}
if (-not (Test-Path -LiteralPath $CompilerPath -PathType Leaf)) { throw '找不到 Inno Setup 编译器。' }
if (& git -C $repo status --porcelain) { throw '请先提交源码，再生成对应发布物。' }
$commit = (& git -C $repo rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw '读取源码提交失败。' }
$stem = "Cap-CN-$version-windows-x64"
foreach ($name in @("$stem-setup.exe", "$stem-portable.zip", 'README.txt')) {
    if (Test-Path -LiteralPath (Join-Path $output $name)) { throw "输出已存在：$name" }
}
$stage = Join-Path $repo ('target\preview-package-' + [Guid]::NewGuid().ToString('N'))
$app = Join-Path $stage 'app'
$null = New-Item -ItemType Directory -Path $app, $output -Force
foreach ($name in @('Cap.exe', 'cap-muxer.exe')) {
    Copy-Item -LiteralPath (Join-Path $package $name) -Destination $app
}
Get-ChildItem -LiteralPath $package -File -Filter '*.dll' | Copy-Item -Destination $app
Copy-Item -LiteralPath (Join-Path $repo 'LICENSE') -Destination (Join-Path $app 'LICENSE-AGPL.txt')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'third-party') -Destination $app -Recurse
$tools = Join-Path $app 'tools'
$source = Join-Path $app 'source-tools'
$null = New-Item -ItemType Directory -Path $tools, $source
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'maintenance.ps1'), (Join-Path $PSScriptRoot 'maintenance-core.psm1') -Destination $tools
Copy-Item -LiteralPath $PSScriptRoot -Destination (Join-Path $source 'zh-cn') -Recurse
Copy-Item -LiteralPath (Join-Path $repo 'BUILDING.zh-CN.md'), (Join-Path $repo 'UPSTREAM.md') -Destination $source
& git -C $repo diff --binary v0.6.1-cn.2 HEAD "--output=$(Join-Path $app '完整源码改动.patch')"
if ($LASTEXITCODE -ne 0) { throw '导出源码补丁失败。' }
$release | Add-Member -NotePropertyName previewTag -NotePropertyValue $tag
$release | Add-Member -NotePropertyName sourceCommit -NotePropertyValue $commit
$release | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $app 'release.json') -Encoding utf8
$notes = "Cap 中文双屏编辑增强预览版 4`r`n发行标签：$tag`r`n源码提交：$commit`r`n完整源码改动.patch 相对 v0.6.1-cn.2 稳定版。`r`n`r`n" +
    [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'releases\dual-preview-4.md'))
[IO.File]::WriteAllText((Join-Path $app '使用说明.txt'), $notes, [Text.UTF8Encoding]::new($true))
[IO.File]::WriteAllText((Join-Path $output 'README.txt'), $notes, [Text.UTF8Encoding]::new($true))
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'installer\安装与迁移说明.txt') -Destination $stage
Copy-Item -LiteralPath (Join-Path $stage '安装与迁移说明.txt') -Destination $app
$launcher = "@echo off`r`npowershell.exe -NoProfile -STA -WindowStyle Hidden -File `"%~dp0tools\maintenance.ps1`"`r`n"
[IO.File]::WriteAllText((Join-Path $app '维护与迁移.cmd'), $launcher, [Text.Encoding]::ASCII)
foreach ($file in Get-ChildItem -LiteralPath $tools -File) {
    [IO.File]::WriteAllText($file.FullName, [IO.File]::ReadAllText($file.FullName), [Text.UTF8Encoding]::new($true))
}
$seed = @{
    general_settings = @{ hasCompletedStartup = $true; hasCompletedOnboarding = $true; autoZoomOnClicks = $true; postStudioRecordingBehaviour = 'openEditor' }
    recording_settings = @{ mode = 'studio' }
} | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $stage 'seed-store.json'), $seed, [Text.UTF8Encoding]::new($false))
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory($app, (Join-Path $output "$stem-portable.zip"), [IO.Compression.CompressionLevel]::Optimal, $false)
& $CompilerPath "/DPayloadDir=$stage" "/DOutputDir=$output" "/DAppVersion=$version" `
    "/DNumericVersion=$($release.numericVersion)" "/DAppId=$($release.appId)" (Join-Path $PSScriptRoot 'installer\cap-cn.iss')
if ($LASTEXITCODE -ne 0) { throw '安装包编译失败，已保留打包目录。' }
Get-ChildItem -LiteralPath $output -File | Select-Object FullName, Length
