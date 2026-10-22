$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-CapDirectory {
    param([Parameter(Mandatory)][string]$AppDirectory)
    $root = [IO.Path]::GetFullPath($AppDirectory).TrimEnd('\', '/')
    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        throw "目录不存在：$root"
    }
    if ((Get-Item -LiteralPath $root -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw '不支持通过目录链接进行迁移，请选择真实安装目录。'
    }
    return $root
}

function Assert-CapClosed {
    param([string]$AppDirectory)
    $exe = Join-Path $AppDirectory 'Cap.exe'
    $running = Get-Process -Name Cap -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and [string]::Equals($_.Path, $exe, [StringComparison]::OrdinalIgnoreCase) }
    if ($running) { throw '请先退出这个目录中的 Cap 中文版，再进行备份或恢复。官方版无需退出。' }
}

function Get-CapProfileItems {
    param([string]$ProfileDirectory)
    $pending = New-Object 'System.Collections.Generic.Stack[string]'
    $pending.Push($ProfileDirectory)
    while ($pending.Count -gt 0) {
        $folder = $pending.Pop()
        foreach ($item in Get-ChildItem -LiteralPath $folder -Force) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "数据目录包含链接，请先单独备份链接目标：$($item.FullName)"
            }
            $relative = $item.FullName.Substring($ProfileDirectory.Length + 1).Replace('\', '/')
            if ($relative -in @('cap-gpui.pid', 'cap-gpui.ipc') -or $relative -eq 'logs') { continue }
            $item
            if ($item.PSIsContainer) { $pending.Push($item.FullName) }
        }
    }
}

function Export-CapProfile {
    param(
        [Parameter(Mandatory)][string]$AppDirectory,
        [Parameter(Mandatory)][string]$ArchivePath
    )
    $root = Get-CapDirectory $AppDirectory
    Assert-CapClosed $root
    $profile = Join-Path $root 'profile'
    if (-not (Test-Path -LiteralPath (Join-Path $profile 'store') -PathType Leaf)) {
        throw '所选目录中没有 Cap 中文版的 profile\store。'
    }
    if ((Get-Item -LiteralPath $profile -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw 'profile 是目录链接，请单独备份其真实目标。'
    }
    $output = [IO.Path]::GetFullPath($ArchivePath)
    if ($output.StartsWith($profile + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw '备份文件不能保存在 profile 里面。'
    }
    if (Test-Path -LiteralPath $output) { throw '备份文件已经存在，请另选名称。' }
    $items = @(Get-CapProfileItems $profile)
    $zip = [IO.Compression.ZipFile]::Open($output, [IO.Compression.ZipArchiveMode]::Create)
    try {
        $metadata = @{
            format = 1
            product = 'Cap-CN'
            createdAt = [DateTime]::UtcNow.ToString('o')
            sourceProfile = $profile
        } | ConvertTo-Json
        $entry = $zip.CreateEntry('cap-cn-backup.json')
        $writer = New-Object IO.StreamWriter ($entry.Open()), ([Text.UTF8Encoding]::new($false))
        try { $writer.Write($metadata) } finally { $writer.Dispose() }
        foreach ($item in $items) {
            $name = 'profile/' + $item.FullName.Substring($profile.Length + 1).Replace('\', '/')
            if ($item.PSIsContainer) {
                $null = $zip.CreateEntry($name + '/')
            } else {
                $null = [IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
                    $zip, $item.FullName, $name, [IO.Compression.CompressionLevel]::Fastest
                )
            }
        }
    } finally { $zip.Dispose() }
    return $output
}

function Assert-CapChildPath {
    param([string]$Root, [string]$Path)
    $absolute = [IO.Path]::GetFullPath($Path)
    if (-not $absolute.StartsWith($Root + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw "迁移目标超出安装目录：$absolute"
    }
}

function Import-CapProfile {
    param(
        [Parameter(Mandatory)][string]$AppDirectory,
        [Parameter(Mandatory)][string]$ArchivePath
    )
    $root = Get-CapDirectory $AppDirectory
    Assert-CapClosed $root
    $profile = Join-Path $root 'profile'
    if ((Test-Path -LiteralPath $profile) -and
        ((Get-Item -LiteralPath $profile -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'profile 是目录链接，不能自动替换。'
    }
    $stage = Join-Path $root ('.restore-' + [Guid]::NewGuid().ToString('N'))
    $stageProfile = Join-Path $stage 'profile'
    $previous = Join-Path $root ('profile-backup-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
    foreach ($path in @($profile, $stage, $stageProfile, $previous)) { Assert-CapChildPath $root $path }
    $zip = [IO.Compression.ZipFile]::OpenRead([IO.Path]::GetFullPath($ArchivePath))
    try {
        $metadataEntry = $zip.GetEntry('cap-cn-backup.json')
        if (-not $metadataEntry -or $metadataEntry.Length -gt 65536) { throw '不是支持的 Cap 中文版备份。' }
        $reader = New-Object IO.StreamReader ($metadataEntry.Open()), ([Text.Encoding]::UTF8)
        try { $metadata = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
        if ($metadata.product -ne 'Cap-CN' -or $metadata.format -ne 1) { throw '备份格式不受支持。' }
        if (-not $zip.GetEntry('profile/store')) { throw '备份缺少 profile/store。' }
        $total = [int64]0
        foreach ($entry in $zip.Entries) {
            if ($entry.FullName -eq 'cap-cn-backup.json') { continue }
            $name = $entry.FullName.Replace('\', '/')
            if (-not $name.StartsWith('profile/', [StringComparison]::Ordinal) -or
                $name -match '(^|/)\.\.?(/|$)|[:\x00]' -or
                (($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000 -or
                ($entry.ExternalAttributes -band 0x400)) {
                throw "备份包含不安全的路径或链接：$name"
            }
            Assert-CapChildPath $stage ([IO.Path]::GetFullPath((Join-Path $stage $name)))
            $total += $entry.Length
        }
        $drive = New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($root))
        if ($drive.IsReady -and $total -gt $drive.AvailableFreeSpace) { throw '磁盘空间不足，无法恢复这份备份。' }
        $null = New-Item -ItemType Directory -Path $stage
        foreach ($entry in $zip.Entries) {
            if ($entry.FullName -eq 'cap-cn-backup.json') { continue }
            $name = $entry.FullName.Replace('\', '/')
            if ($name -in @('profile/cap-gpui.pid', 'profile/cap-gpui.ipc') -or $name.StartsWith('profile/logs/')) { continue }
            $target = [IO.Path]::GetFullPath((Join-Path $stage $name))
            if ($name.EndsWith('/')) {
                $null = New-Item -ItemType Directory -Path $target -Force
            } else {
                $null = New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force
                [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $target, $false)
            }
        }
    } finally { $zip.Dispose() }
    $storePath = Join-Path $stageProfile 'store'
    $store = [IO.File]::ReadAllText($storePath) | ConvertFrom-Json
    $settings = $store.general_settings
    if ($settings -and $metadata.sourceProfile) {
        $old = [string]$metadata.sourceProfile
        $recordings = [string]$settings.recordingsPath
        $changed = $false
        if ($recordings.StartsWith($old + '\', [StringComparison]::OrdinalIgnoreCase)) {
            $settings.recordingsPath = $profile + $recordings.Substring($old.Length)
            $changed = $true
        }
        if ($settings.previousRecordingsPaths) {
            $settings.previousRecordingsPaths = @(
                foreach ($path in $settings.previousRecordingsPaths) {
                    if ($path.StartsWith($old + '\', [StringComparison]::OrdinalIgnoreCase)) {
                        $profile + $path.Substring($old.Length)
                        $changed = $true
                    } else { $path }
                }
            )
        }
        if ($changed) {
            [IO.File]::WriteAllText($storePath, ($store | ConvertTo-Json -Depth 100), [Text.UTF8Encoding]::new($false))
        }
    }
    if (Test-Path -LiteralPath $profile) { Move-Item -LiteralPath $profile -Destination $previous }
    try { Move-Item -LiteralPath $stageProfile -Destination $profile }
    catch {
        if ((Test-Path -LiteralPath $previous) -and -not (Test-Path -LiteralPath $profile)) {
            Move-Item -LiteralPath $previous -Destination $profile
        }
        throw
    }
    return [pscustomobject]@{ Profile = $profile; PreviousProfile = $previous }
}

function Get-CapOfficialUpdate {
    param([Parameter(Mandatory)][string]$AppDirectory)
    $root = Get-CapDirectory $AppDirectory
    $release = [IO.File]::ReadAllText((Join-Path $root 'release.json')) | ConvertFrom-Json
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $releases = Invoke-RestMethod -Uri 'https://api.github.com/repos/CapSoftware/Cap/releases?per_page=20' `
        -Headers @{ 'User-Agent' = 'Cap-CN-Update-Check'; Accept = 'application/vnd.github+json' } -TimeoutSec 20
    $latest = $releases | Where-Object { -not $_.draft -and -not $_.prerelease -and $_.tag_name -match '^cap-v\d+\.\d+\.\d+$' } |
        Select-Object -First 1
    if (-not $latest) { throw '没有找到官方桌面版的稳定发布。' }
    $installed = [version]($release.upstreamTag -replace '^cap-v', '')
    $upstream = [version]($latest.tag_name -replace '^cap-v', '')
    return [pscustomobject]@{
        Installed = $release.version
        UpstreamTag = $release.upstreamTag
        Latest = $latest.tag_name
        HasUpdate = $upstream -gt $installed
        ReleaseUrl = $latest.html_url
    }
}

Export-ModuleMember -Function Export-CapProfile, Import-CapProfile, Get-CapOfficialUpdate
