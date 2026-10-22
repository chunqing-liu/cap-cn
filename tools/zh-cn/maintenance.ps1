param(
    [ValidateSet('Menu', 'Export', 'Import', 'CheckUpdates')][string]$Action = 'Menu',
    [string]$AppDirectory = (Split-Path -Parent $PSScriptRoot),
    [string]$ArchivePath,
    [switch]$Quiet
)
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'maintenance-core.psm1') -Force
Add-Type -AssemblyName System.Windows.Forms
[Windows.Forms.Application]::EnableVisualStyles()

function Invoke-CapMaintenance {
    param([string]$SelectedAction)
    switch ($SelectedAction) {
        'Export' {
            $path = $ArchivePath
            if (-not $path) {
                $dialog = New-Object Windows.Forms.SaveFileDialog
                $dialog.Filter = 'Cap 中文版备份 (*.zip)|*.zip'
                $dialog.FileName = 'Cap-CN-备份-' + (Get-Date -Format yyyyMMdd-HHmmss) + '.zip'
                $dialog.OverwritePrompt = $true
                if ($dialog.ShowDialog() -ne 'OK') { return }
                $path = $dialog.FileName
            }
            $result = Export-CapProfile -AppDirectory $AppDirectory -ArchivePath $path
            if (-not $Quiet) {
                $null = [Windows.Forms.MessageBox]::Show("备份已保存：`n$result`n`n仅包含 profile 内的数据；外部录屏目录和素材请另行备份。备份可能包含私密数据，请妥善保存。", '备份完成')
            }
            return $result
        }
        'Import' {
            $path = $ArchivePath
            if (-not $path) {
                $dialog = New-Object Windows.Forms.OpenFileDialog
                $dialog.Filter = 'Cap 中文版备份 (*.zip)|*.zip'
                if ($dialog.ShowDialog() -ne 'OK') { return }
                $path = $dialog.FileName
            }
            if (-not $Quiet -and [Windows.Forms.MessageBox]::Show(
                '请先退出这个目录中的中文版。现有数据将保留为独立备份目录，是否恢复所选备份？只导入可信来源的备份。',
                '导入备份', 'YesNo', 'Warning'
            ) -ne 'Yes') { return }
            $result = Import-CapProfile -AppDirectory $AppDirectory -ArchivePath $path
            if (-not $Quiet) {
                $null = [Windows.Forms.MessageBox]::Show("恢复完成。`n数据：$($result.Profile)`n旧数据：$($result.PreviousProfile)`n`n现在可以重新打开中文版。", '恢复完成')
            }
            return $result
        }
        'CheckUpdates' {
            $result = Get-CapOfficialUpdate -AppDirectory $AppDirectory
            if (-not $Quiet) {
                $state = if ($result.HasUpdate) { '发现官方新版本，需要重新适配汉化后再升级。' } else { '当前汉化所基于的官方版本已是最新稳定版。' }
                $null = [Windows.Forms.MessageBox]::Show(
                    "当前中文版：$($result.Installed)`n所基于的官方版：$($result.UpstreamTag)`n官方最新：$($result.Latest)`n`n$state`n`n本工具不会下载安装官方版，也不会自动替换汉化。`n发布说明：$($result.ReleaseUrl)",
                    '检查官方更新'
                )
            }
            return $result
        }
    }
}

try {
    if ($Action -ne 'Menu') { Invoke-CapMaintenance $Action; exit 0 }
    $form = New-Object Windows.Forms.Form
    $form.Text = 'Cap 中文版 · 维护与迁移'
    $form.ClientSize = New-Object Drawing.Size 520, 300
    $form.StartPosition = 'CenterScreen'
    $form.FormBorderStyle = 'FixedDialog'
    $form.MaximizeBox = $false
    $form.Font = New-Object Drawing.Font 'Microsoft YaHei UI', 10
    $label = New-Object Windows.Forms.Label
    $label.Text = "请先退出中文版，再进行备份或恢复。`n只处理当前安装目录，不改动官方版。"
    $label.SetBounds(24, 20, 472, 54)
    $form.Controls.Add($label)
    $buttons = @(
        @{ Text = '导出迁移备份'; Action = 'Export' },
        @{ Text = '导入迁移备份'; Action = 'Import' },
        @{ Text = '检查官方更新'; Action = 'CheckUpdates' }
    )
    $y = 86
    foreach ($item in $buttons) {
        $button = New-Object Windows.Forms.Button
        $button.Text = $item.Text
        $button.Tag = $item.Action
        $button.SetBounds(24, $y, 472, 42)
        $button.Add_Click({
            param($sender, $event)
            $sender.Enabled = $false
            try { $null = Invoke-CapMaintenance $sender.Tag }
            catch { $null = [Windows.Forms.MessageBox]::Show($_.Exception.Message, '操作未完成', 'OK', 'Error') }
            finally { $sender.Enabled = $true }
        })
        $form.Controls.Add($button)
        $y += 52
    }
    $note = New-Object Windows.Forms.Label
    $note.Text = '官方更新不会自动变成汉化版，需重新适配后安装。'
    $note.SetBounds(24, 250, 472, 32)
    $form.Controls.Add($note)
    $null = $form.ShowDialog()
} catch {
    if (-not $Quiet) { $null = [Windows.Forms.MessageBox]::Show($_.Exception.Message, '操作未完成', 'OK', 'Error') }
    else { Write-Error $_ }
    exit 1
}
