param([string]$Destination)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
if (-not $Destination) { $Destination = Join-Path (Split-Path -Parent $repo) 'Cap-cn-gpui' }
$target = [IO.Path]::GetFullPath($Destination)
if (Test-Path -LiteralPath $target) { throw '目标目录已存在，不会改动。请选择一个不存在的新目录。' }
$revision = '5d1f83d9f27a19bec1fb241dc33b42238af9cf8d'
$patch = Join-Path $PSScriptRoot 'gpui-local.patch'
if (-not (Test-Path -LiteralPath $patch)) { throw '缺少 gpui-local.patch。' }
function Invoke-GpuiGit {
    param([string[]]$Arguments)
    & git -C $target @Arguments
    if ($LASTEXITCODE -ne 0) { throw "准备 GPUI 失败，已保留目录：$target" }
}
$null = New-Item -ItemType Directory -Path $target
Invoke-GpuiGit -Arguments @('init')
Invoke-GpuiGit -Arguments @('remote', 'add', 'origin', 'https://github.com/wingleeio/zed.git')
Invoke-GpuiGit -Arguments @('fetch', '--depth', '1', 'origin', $revision)
Invoke-GpuiGit -Arguments @('checkout', '--detach', 'FETCH_HEAD')
Invoke-GpuiGit -Arguments @('apply', '--check', $patch)
Invoke-GpuiGit -Arguments @('apply', $patch)
Write-Output "GPUI 源码准备完成：$target"
Write-Output '已包含当前版本所需的完整框架补丁；不要再重复应用上游的四个 GPUI 补丁。'
