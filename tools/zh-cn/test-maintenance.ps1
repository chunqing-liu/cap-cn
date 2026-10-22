$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'maintenance-core.psm1') -Force
$root = Join-Path ([IO.Path]::GetTempPath()) ('cap-cn-maintenance-test-' + [Guid]::NewGuid().ToString('N'))
$source = Join-Path $root '旧电脑 中文目录'
$target = Join-Path $root '新电脑'
$null = New-Item -ItemType Directory -Path (Join-Path $source 'profile\recordings\测试.cap'), (Join-Path $source 'profile\logs'), (Join-Path $target 'profile') -Force
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $source 'profile\store'), (@{
    general_settings = @{
        recordingsPath = (Join-Path $source 'profile\recordings')
        previousRecordingsPaths = @((Join-Path $source 'profile\old-recordings'), 'D:\ExternalRecordings')
    }
} | ConvertTo-Json), $utf8)
[IO.File]::WriteAllText((Join-Path $source 'profile\recordings\测试.cap\video.txt'), '测试视频内容', $utf8)
[IO.File]::WriteAllText((Join-Path $source 'profile\cap-gpui.pid'), '123', $utf8)
[IO.File]::WriteAllText((Join-Path $source 'profile\logs\private.log'), 'private', $utf8)
[IO.File]::WriteAllText((Join-Path $target 'profile\store'), '{"existing":true}', $utf8)
$archive = Join-Path $root 'backup.zip'
$null = Export-CapProfile $source $archive
$result = Import-CapProfile $target $archive
if ([IO.File]::ReadAllText((Join-Path $target 'profile\recordings\测试.cap\video.txt')) -ne '测试视频内容') { throw '录屏内容没有正确恢复。' }
if ([IO.File]::ReadAllText((Join-Path $result.PreviousProfile 'store')) -ne '{"existing":true}') { throw '原数据没有保留。' }
if (Test-Path -LiteralPath (Join-Path $target 'profile\cap-gpui.pid')) { throw '不应导入进程状态。' }
if (Test-Path -LiteralPath (Join-Path $target 'profile\logs\private.log')) { throw '不应导入日志。' }
$restored = [IO.File]::ReadAllText((Join-Path $target 'profile\store')) | ConvertFrom-Json
if ($restored.general_settings.recordingsPath -ne (Join-Path $target 'profile\recordings')) { throw '内部录屏路径没有适应新电脑。' }
if ($restored.general_settings.previousRecordingsPaths[0] -ne (Join-Path $target 'profile\old-recordings') -or
    $restored.general_settings.previousRecordingsPaths[1] -ne 'D:\ExternalRecordings') { throw '历史录屏路径迁移不正确。' }
$rejected = $false
try { Export-CapProfile $source $archive } catch { $rejected = $true }
if (-not $rejected) { throw '备份不应覆盖已有文件。' }
$bad = Join-Path $root 'unsafe.zip'
$zip = [IO.Compression.ZipFile]::Open($bad, [IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($pair in @(
        @('cap-cn-backup.json', '{"format":1,"product":"Cap-CN"}'),
        @('profile/store', '{}'),
        @('profile/../../outside.txt', 'unsafe')
    )) {
        $entry = $zip.CreateEntry($pair[0])
        $writer = New-Object IO.StreamWriter ($entry.Open())
        try { $writer.Write($pair[1]) } finally { $writer.Dispose() }
    }
} finally { $zip.Dispose() }
$rejected = $false
try { Import-CapProfile $target $bad } catch { $rejected = $true }
if (-not $rejected -or (Test-Path -LiteralPath (Join-Path $root 'outside.txt'))) { throw '不安全的压缩路径没有被拒绝。' }
if (-not (Test-Path -LiteralPath (Join-Path $target 'profile\recordings\测试.cap\video.txt'))) { throw '失败的导入破坏了原数据。' }
Write-Output "PASS: backup, restore, Unicode paths, retained old data, transient exclusion, path remapping, no overwrite, zip traversal rejection."
Write-Output "Test artifacts retained: $root"
