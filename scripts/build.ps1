# ============================================================
# LibPool 构建脚本 (Windows / PowerShell)
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
# 1. 交叉编译 libman (aarch64-unknown-linux-musl 纯静态)
# 2. 组装模块目录结构
# 3. 打包 libpool-<version>.zip
#
# 前置：rustup + aarch64-unknown-linux-musl target
#   rustup toolchain install stable --profile minimal
#   rustup target add aarch64-unknown-linux-musl
#   可选：rustup target add armv7-unknown-linux-musleabihf（32 位兼容）
# ============================================================
# 注意：不设置 $ErrorActionPreference="Stop"，否则 cargo 的 stderr 警告会被当作终止错误。
# 关键失败点已用 $LASTEXITCODE / Test-Path 显式检查。

$ROOT = Split-Path -Parent $PSScriptRoot
$CRATE = Join-Path $ROOT "src\rust-libman"
$MODULE_DIR = Join-Path $ROOT "module"
$TARGET = "aarch64-unknown-linux-musl"

Write-Host "==> [1/4] 编译 libman ($TARGET) ..." -ForegroundColor Cyan
# 必须在 crate 目录内执行，否则找不到 .cargo/config.toml（rust-lld 链接器配置）
# 2>&1 合并 stderr：避免 cargo 的编译警告在 $ErrorActionPreference=Stop 下被当作终止错误
Push-Location $CRATE
try {
  & cargo build --release --target $TARGET 2>&1 | Out-Host
} finally {
  Pop-Location
}
if ($LASTEXITCODE -ne 0) { throw "cargo build 失败" }

$BIN = Join-Path $CRATE "target\$TARGET\release\libman.exe"
if (-not (Test-Path $BIN)) { $BIN = Join-Path $CRATE "target\$TARGET\release\libman" }
Write-Host "    生成: $BIN" -ForegroundColor Green

Write-Host "==> [2/4] 组装模块目录 ..." -ForegroundColor Cyan
if (Test-Path $MODULE_DIR) { Remove-Item $MODULE_DIR -Recurse -Force }
New-Item -ItemType Directory -Path $MODULE_DIR | Out-Null

# 复制模块脚本与元数据
Copy-Item (Join-Path $ROOT "module.prop")      $MODULE_DIR
Copy-Item (Join-Path $ROOT "customize.sh")     $MODULE_DIR
Copy-Item (Join-Path $ROOT "service.sh")       $MODULE_DIR
Copy-Item (Join-Path $ROOT "post-fs-data.sh")  $MODULE_DIR
Copy-Item (Join-Path $ROOT "uninstall.sh")     $MODULE_DIR
Copy-Item (Join-Path $ROOT "sepolicy.rule")    $MODULE_DIR

# 复制 WebUI
Copy-Item (Join-Path $ROOT "webroot")          (Join-Path $MODULE_DIR "webroot") -Recurse
# 复制仓库数据到 webroot/data
New-Item -ItemType Directory -Path (Join-Path $MODULE_DIR "webroot\data") -Force | Out-Null
Copy-Item (Join-Path $ROOT "repo_src\repos.json") (Join-Path $MODULE_DIR "webroot\data\repos.json")
# 初始状态文件
Copy-Item (Join-Path $ROOT "repo_src\state.json") (Join-Path $MODULE_DIR "webroot\data\state.json") -ErrorAction SilentlyContinue

# 复制二进制与兜底脚本
New-Item -ItemType Directory -Path (Join-Path $MODULE_DIR "tools") -Force | Out-Null
Copy-Item $BIN (Join-Path $MODULE_DIR "tools\libman")
Copy-Item (Join-Path $ROOT "tools\libman.sh") (Join-Path $MODULE_DIR "tools\libman.sh") -ErrorAction SilentlyContinue
# 32 位（armv7）兼容二进制
if (Test-Path (Join-Path $ROOT "tools\libman-arm")) {
  Copy-Item (Join-Path $ROOT "tools\libman-arm") (Join-Path $MODULE_DIR "tools\libman-arm")
}

# 公告/云更新配置目录（源码用 module-config/ 避免与 git 的 .git 冲突；打包进模块后名为 .git/）
New-Item -ItemType Directory -Path (Join-Path $MODULE_DIR ".git") -Force | Out-Null
if (Test-Path (Join-Path $ROOT "module-config\url.txt")) {
  Copy-Item (Join-Path $ROOT "module-config\url.txt") (Join-Path $MODULE_DIR ".git\url.txt")
}
# 云更新配置（可选，用户可手动编辑；缺省时 WebUI 回退 app.js 内置常量）
foreach ($f in @("update.txt", "download.txt")) {
  if (Test-Path (Join-Path $ROOT "module-config\$f")) {
    Copy-Item (Join-Path $ROOT "module-config\$f") (Join-Path $MODULE_DIR ".git\$f")
  }
}

# 创建空目录占位（magic mount 目标）
New-Item -ItemType Directory -Path (Join-Path $MODULE_DIR "system\bin") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $MODULE_DIR "system\lib") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $MODULE_DIR "libs") -Force | Out-Null

Write-Host "==> [3/4] 打包 zip ..." -ForegroundColor Cyan
$VER = (Select-String -Path (Join-Path $ROOT "module.prop") -Pattern '^version=(.+)').Matches.Groups[1].Value
$ZIP = Join-Path $ROOT "dist\libpool-$VER.zip"
New-Item -ItemType Directory -Path (Join-Path $ROOT "dist") -Force | Out-Null
if (Test-Path $ZIP) { Remove-Item $ZIP -Force }

# 用 .NET 的 ZipFile 打包（确保 libman 二进制保留可执行语义由刷机工具处理）
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($MODULE_DIR, $ZIP)

Write-Host "==> [4/4] 完成: $ZIP" -ForegroundColor Green
Write-Host "    模块目录: $MODULE_DIR"
