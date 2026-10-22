param(
    [string]$PackageDirectory,
    [string]$CompilerPath = 'E:\Agentspace\dev-tools\innosetup-6.4.3\ISCC.exe',
    [string]$CrtDirectory = 'E:\VisualStudio\2022\BuildTools\VC\Redist\MSVC\14.44.35112\x64\Microsoft.VC143.CRT',
    [string]$GpuiDirectory,
    [string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$release = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'release.json')) | ConvertFrom-Json
if (-not $GpuiDirectory) { $GpuiDirectory = Join-Path (Split-Path -Parent $repo) 'Cap-cn-gpui' }
if (-not $PackageDirectory) { $PackageDirectory = Join-Path $repo ('dist\Cap-' + $release.version) }
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repo 'dist\installers' }
$package = [IO.Path]::GetFullPath($PackageDirectory)
$output = [IO.Path]::GetFullPath($OutputDirectory)
if ((Get-Item -LiteralPath (Join-Path $package 'Cap.exe')).VersionInfo.ProductVersion -ne $release.version) {
    throw '主程序版本与 release.json 不一致，请先构建并整理对应版本。'
}
foreach ($file in @($CompilerPath, (Join-Path $CrtDirectory 'msvcp140.dll'), (Join-Path $CrtDirectory 'msvcp140_1.dll'),
        (Join-Path $CrtDirectory 'vcruntime140.dll'), (Join-Path $CrtDirectory 'vcruntime140_1.dll'))) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "缺少打包依赖：$file" }
}
$stem = 'Cap-CN-' + $release.version + '-windows-x64'
foreach ($name in @("$stem-setup.exe", "$stem-portable.zip", "$stem-maintenance.zip")) {
    if (Test-Path -LiteralPath (Join-Path $output $name)) { throw "已存在输出文件，请另选输出目录：$name" }
}
$stage = Join-Path $repo ('target\installer-' + $release.version + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
$app = Join-Path $stage 'app'
$null = New-Item -ItemType Directory -Path $app, $output -Force
Get-ChildItem -LiteralPath $package -File |
    Where-Object { $_.Extension -eq '.dll' -or $_.Name -in @('Cap.exe', 'cap-muxer.exe', 'LICENSE-AGPL.txt', '使用说明.txt') } |
    Copy-Item -Destination $app
Get-ChildItem -LiteralPath $CrtDirectory -File -Filter '*.dll' | Copy-Item -Destination $app
$licenseSource = Join-Path $PSScriptRoot 'third-party'
if (Test-Path -LiteralPath $licenseSource) {
    Copy-Item -LiteralPath $licenseSource -Destination (Join-Path $app 'third-party') -Recurse
}
$tools = Join-Path $app 'tools'
$null = New-Item -ItemType Directory -Path $tools
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'maintenance.ps1'), (Join-Path $PSScriptRoot 'maintenance-core.psm1') -Destination $tools
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'release.json') -Destination $app
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'installer\安装与迁移说明.txt') -Destination $app
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'installer\安装与迁移说明.txt') -Destination $stage
$usagePath = Join-Path $app '使用说明.txt'
$usage = [IO.File]::ReadAllText($usagePath)
$usage = "本包补充说明：`r`n安装版请通过 Windows 设置中的已安装的应用卸载，profile 数据会保留。`r`n免安装版可在退出后删除程序目录，删除前请另行备份 profile。`r`n安装、迁移与升级请优先阅读同目录的 安装与迁移说明.txt。`r`n下文的本机源码路径仅用于标明原构建环境，换电脑后可用随包补丁与 source-tools 还原源码。`r`n`r`n" + $usage
[IO.File]::WriteAllText($usagePath, $usage, [Text.UTF8Encoding]::new($true))
$sourceTools = Join-Path $app 'source-tools'
$null = New-Item -ItemType Directory -Path $sourceTools
Get-ChildItem -LiteralPath $PSScriptRoot -Force | Copy-Item -Destination $sourceTools -Recurse
$null = & git -C $repo rev-parse --verify "$($release.upstreamTag)^{commit}" 2>$null
if ($LASTEXITCODE -eq 0) {
    & git -C $repo diff --binary $release.upstreamTag "--output=$(Join-Path $app 'source-changes.patch')" -- apps/desktop-gpui
    if ($LASTEXITCODE -ne 0) { throw '无法导出源码补丁。' }
} else {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'upstream-changes.patch') -Destination (Join-Path $app 'source-changes.patch')
}
@"
Microsoft Visual C++ 2022 x64 app-local runtime
来源：Visual Studio Build Tools 的可再发行目录 Microsoft.VC143.CRT
版本：$((Get-Item -LiteralPath (Join-Path $CrtDirectory 'vcruntime140.dll')).VersionInfo.FileVersion)
未修改这些微软运行库文件；安装在应用目录，不覆盖系统运行库。
说明：https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files
FFmpeg、DXC、ONNX 和 cap-muxer 的来源另见 使用说明.txt。
"@ | Set-Content -LiteralPath (Join-Path $app '运行库来源.txt') -Encoding UTF8
$seed = @{
    general_settings = @{
        hasCompletedStartup = $true
        hasCompletedOnboarding = $true
        autoZoomOnClicks = $true
        postStudioRecordingBehaviour = 'openEditor'
    }
    recording_settings = @{ mode = 'studio' }
} | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $stage 'seed-store.json'), $seed, [Text.UTF8Encoding]::new($false))
$launcher = "@echo off`r`npowershell.exe -NoProfile -STA -WindowStyle Hidden -ExecutionPolicy Bypass -File `"%~dp0tools\maintenance.ps1`"`r`n"
[IO.File]::WriteAllText((Join-Path $app '维护与迁移.cmd'), $launcher, [Text.Encoding]::ASCII)
foreach ($file in Get-ChildItem -LiteralPath $app -Recurse -File |
        Where-Object { $_.Extension -in @('.ps1', '.psm1', '.txt') }) {
    $text = [IO.File]::ReadAllText($file.FullName)
    [IO.File]::WriteAllText($file.FullName, $text, [Text.UTF8Encoding]::new($true))
}
$infoPath = Join-Path $stage '安装与迁移说明.txt'
[IO.File]::WriteAllText($infoPath, [IO.File]::ReadAllText($infoPath), [Text.UTF8Encoding]::new($true))
$kit = Join-Path $stage 'maintenance-kit'
$null = New-Item -ItemType Directory -Path (Join-Path $kit 'tools\zh-cn') -Force
Get-ChildItem -LiteralPath $sourceTools -Force | Copy-Item -Destination (Join-Path $kit 'tools\zh-cn') -Recurse
Copy-Item -LiteralPath (Join-Path $app 'source-changes.patch'), (Join-Path $app 'LICENSE-AGPL.txt') -Destination $kit
@{
    repository = 'https://github.com/CapSoftware/Cap.git'
    baseTag = $release.upstreamTag
    baseCommit = $release.upstreamCommit
    chineseRepository = 'https://github.com/chunqing-liu/cap-cn'
    gpuiRepository = 'https://github.com/wingleeio/zed.git'
    gpuiCommit = (& git -C $GpuiDirectory rev-parse HEAD).Trim()
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $kit 'source.json') -Encoding UTF8
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory($app, (Join-Path $output "$stem-portable.zip"), [IO.Compression.CompressionLevel]::Fastest, $false)
$zip = [IO.Compression.ZipFile]::Open((Join-Path $output "$stem-portable.zip"), [IO.Compression.ZipArchiveMode]::Update)
try {
    $entry = $zip.CreateEntry('profile/store')
    $writer = New-Object IO.StreamWriter ($entry.Open()), ([Text.UTF8Encoding]::new($false))
    try { $writer.Write($seed) } finally { $writer.Dispose() }
    $null = $zip.CreateEntry('profile/recordings/')
} finally { $zip.Dispose() }
[IO.Compression.ZipFile]::CreateFromDirectory($kit, (Join-Path $output "$stem-maintenance.zip"), [IO.Compression.CompressionLevel]::Optimal, $false)
& $CompilerPath "/DPayloadDir=$stage" "/DOutputDir=$output" "/DAppVersion=$($release.version)" `
    "/DNumericVersion=$($release.numericVersion)" "/DAppId=$($release.appId)" (Join-Path $PSScriptRoot 'installer\cap-cn.iss')
if ($LASTEXITCODE -ne 0) { throw '安装包编译失败。已保留打包目录，便于检查。' }
Get-ChildItem -LiteralPath $output -File | Select-Object FullName, Length
