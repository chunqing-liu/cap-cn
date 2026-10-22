param(
    [Parameter(Mandatory)][ValidatePattern('^cap-v\d+\.\d+\.\d+$')][string]$Ref,
    [Parameter(Mandatory)][string]$RepositoryPath,
    [Parameter(Mandatory)][string]$Destination,
    [Parameter(Mandatory)][string]$GpuiDirectory,
    [Parameter(Mandatory)][string]$PatchPath,
    [ValidateRange(1, 9999)][int]$CnRevision = 1,
    [string]$NodePath = 'node'
)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath($RepositoryPath)
$target = [IO.Path]::GetFullPath($Destination)
$patch = (Resolve-Path -LiteralPath $PatchPath).Path
if (Test-Path -LiteralPath $target) { throw '适配目录已存在，不会覆盖。请另选一个新的目录。' }
function Invoke-CapGit {
    param([string[]]$GitArguments)
    & git -C $repo @GitArguments
    if ($LASTEXITCODE -ne 0) { throw 'Git 操作失败，已保留现有文件；没有重置原工作区。' }
}
Invoke-CapGit -GitArguments @('fetch', 'origin', "refs/tags/${Ref}:refs/tags/${Ref}")
$manifest = (& git -C $repo show "${Ref}:apps/desktop-gpui/Cargo.toml") -join "`n"
if ($LASTEXITCODE -ne 0) { throw '新版本没有预期的 GPUI 桌面项目，需要人工适配。' }
$expected = [regex]::Match($manifest, '(?s)gpui\s*=\s*\{.*?rev\s*=\s*"([0-9a-f]+)"').Groups[1].Value
$gpui = [IO.Path]::GetFullPath($GpuiDirectory)
$actual = (& git -C $gpui rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or -not $expected -or -not $actual.StartsWith($expected)) {
    throw '界面框架版本与新官方源码不匹配。请先按新版本准备 GPUI 源码及官方补丁，再继续。'
}
Invoke-CapGit -GitArguments @('worktree', 'add', '--detach', $target, $Ref)
& git -C $target apply --3way $patch
if ($LASTEXITCODE -ne 0) {
    throw "汉化补丁与新版本存在冲突，请在 $target 人工解决。原来的中文工作区没有被改动。"
}
$newTools = Join-Path $target 'tools\zh-cn'
$null = New-Item -ItemType Directory -Path $newTools -Force
Get-ChildItem -LiteralPath $PSScriptRoot -Force | Copy-Item -Destination $newTools -Recurse -Force
$upstreamVersion = $Ref -replace '^cap-v', ''
$version = "$upstreamVersion-cn.$CnRevision"
$configPath = Join-Path $newTools 'release.json'
$config = [IO.File]::ReadAllText($configPath) | ConvertFrom-Json
$config.version = $version
$config.numericVersion = "$upstreamVersion.$CnRevision"
$config.upstreamTag = $Ref
$config.upstreamCommit = (& git -C $repo rev-parse $Ref).Trim()
[IO.File]::WriteAllText($configPath, ($config | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
$cargoPath = Join-Path $target 'apps\desktop-gpui\Cargo.toml'
$cargo = [IO.File]::ReadAllText($cargoPath)
$cargo = ([regex]'(?m)^version = "[^"]+"').Replace($cargo, ('version = "' + $version + '"'), 1)
$gpuiPath = $gpui.Replace('\', '/')
$cargo = $cargo.Replace('../../../Cap-cn-gpui/crates/', "$gpuiPath/crates/")
[IO.File]::WriteAllText($cargoPath, $cargo, [Text.UTF8Encoding]::new($false))
Push-Location $target
try {
    & $NodePath (Join-Path $newTools 'localize-ui.mjs') --write --list *> (Join-Path $target 'zh-cn-review.txt')
    if ($LASTEXITCODE -ne 0) { throw '汉化脚本失败，请检查 zh-cn-review.txt。' }
    & $NodePath --test (Join-Path $newTools 'localize-ui.test.mjs')
    if ($LASTEXITCODE -ne 0) { throw '汉化工具测试未通过，请先修复。' }
} finally { Pop-Location }
Write-Output "已准备适配目录：$target"
Write-Output "请审阅 zh-cn-review.txt，处理新文案和兼容性问题，再构建 $version。"
Write-Output '这只是准备源码，不代表新版本已完成汉化、构建或实机验证。'
