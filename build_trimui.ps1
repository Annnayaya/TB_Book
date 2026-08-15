# Windows PowerShell 一键打包与安装辅助脚本
param(
    [string]$SdCardDrive = ""
)

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  BrickReader - Trimui Brick 掌机安装包打包工具" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

$packageDir = "package\Apps\BrickReader"
New-Item -ItemType Directory -Force -Path "$packageDir\bin", "$packageDir\books" | Out-Null

Write-Host "[1/3] 检查安装包结构..." -ForegroundColor Yellow
if (Test-Path "$packageDir\config.json") {
    Write-Host "  ✓ config.json 已就绪" -ForegroundColor Green
}
if (Test-Path "$packageDir\launch.sh") {
    Write-Host "  ✓ launch.sh 已就绪" -ForegroundColor Green
}
if (Test-Path "$packageDir\icon.png") {
    Write-Host "  ✓ icon.png 图标已就绪" -ForegroundColor Green
}

# 复制 sample books
Copy-Item "books\*" -Destination "$packageDir\books" -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "`n[2/3] 安装包路径：" -ForegroundColor Yellow
Write-Host "  $((Get-Item $packageDir).FullName)" -ForegroundColor White

if ($SdCardDrive -ne "") {
    $targetPath = "$SdCardDrive\Apps\BrickReader"
    Write-Host "`n[3/3] 正在复制到 TF 卡 ($targetPath)..." -ForegroundColor Yellow
    Copy-Item $packageDir -Destination "$SdCardDrive\Apps" -Recurse -Force
    Write-Host "  ✓ 复制完成！请将 TF 卡插回 Trimui Brick 掌机开机使用。" -ForegroundColor Green
} else {
    Write-Host "`n[3/3] 安装指引：" -ForegroundColor Yellow
    Write-Host "  1. 编译 Linux ARM64 二进制放入 $packageDir\bin\brick_reader"
    Write-Host "  2. 将整个 $packageDir 文件夹复制到 TF 卡的 Apps\ 目录下"
    Write-Host "  3. 插回掌机，在【应用 / Apps】标签页点击【砖读 BrickReader】启动！"
}
