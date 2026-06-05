# ClipMaster 构建并复制 exe 到根目录

# 构建 exe
Write-Host "开始构建 ClipMaster..." -ForegroundColor Green
npm run tauri:build

if ($LASTEXITCODE -eq 0) {
    Write-Host "构建成功！" -ForegroundColor Green

    # 源文件路径
    $sourceExe = "src-tauri\target\release\clipmaster.exe"

    # 目标路径（项目根目录）
    $targetExe = "clipmaster.exe"

    # 复制 exe 到根目录
    if (Test-Path $sourceExe) {
        Copy-Item $sourceExe $targetExe -Force
        Write-Host "✓ exe 已复制到根目录: $targetExe" -ForegroundColor Green

        # 显示文件信息
        $fileInfo = Get-Item $targetExe
        Write-Host ""
        Write-Host "文件信息:" -ForegroundColor Cyan
        Write-Host "  路径: $($fileInfo.FullName)" -ForegroundColor White
        Write-Host "  大小: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor White
        Write-Host ""
        Write-Host "现在可以直接运行: .\clipmaster.exe" -ForegroundColor Yellow
    } else {
        Write-Host "✗ 错误: 找不到构建的 exe 文件" -ForegroundColor Red
    }
} else {
    Write-Host "✗ 构建失败" -ForegroundColor Red
}
