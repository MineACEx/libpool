#!/bin/sh
# ============================================================
# LibPool 构建脚本 (Linux / macOS / WSL)
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
# 1. 交叉编译 libman (aarch64-unknown-linux-musl 纯静态)
# 2. 组装模块目录结构
# 3. 打包 libpool-<version>.zip
#
# 前置：rustup + aarch64-unknown-linux-musl target
# ============================================================
set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/src/rust-libman"
MODULE_DIR="$ROOT/module"
TARGET="aarch64-unknown-linux-musl"

echo "==> [1/4] 编译 libman ($TARGET) ..."
# 必须在 crate 目录内执行，否则找不到 .cargo/config.toml（rust-lld 链接器配置）
( cd "$CRATE" && cargo build --release --target "$TARGET" )
BIN="$CRATE/target/$TARGET/release/libman"
echo "    生成: $BIN"

echo "==> [2/4] 组装模块目录 ..."
rm -rf "$MODULE_DIR"
mkdir -p "$MODULE_DIR"

cp "$ROOT/module.prop" "$MODULE_DIR/"
cp "$ROOT/customize.sh" "$MODULE_DIR/"
cp "$ROOT/service.sh" "$MODULE_DIR/"
cp "$ROOT/post-fs-data.sh" "$MODULE_DIR/"
cp "$ROOT/uninstall.sh" "$MODULE_DIR/"
cp "$ROOT/sepolicy.rule" "$MODULE_DIR/"

cp -r "$ROOT/webroot" "$MODULE_DIR/webroot"
mkdir -p "$MODULE_DIR/webroot/data"
cp "$ROOT/repo_src/repos.json" "$MODULE_DIR/webroot/data/repos.json"
[ -f "$ROOT/repo_src/state.json" ] && cp "$ROOT/repo_src/state.json" "$MODULE_DIR/webroot/data/state.json"

mkdir -p "$MODULE_DIR/tools"
cp "$BIN" "$MODULE_DIR/tools/libman"
[ -f "$ROOT/tools/libman.sh" ] && cp "$ROOT/tools/libman.sh" "$MODULE_DIR/tools/libman.sh"

mkdir -p "$MODULE_DIR/system/bin" "$MODULE_DIR/system/lib" "$MODULE_DIR/libs"

echo "==> [3/4] 打包 zip ..."
VER="$(grep '^version=' "$ROOT/module.prop" | cut -d= -f2)"
mkdir -p "$ROOT/dist"
ZIP="$ROOT/dist/libpool-$VER.zip"
rm -f "$ZIP"
(cd "$MODULE_DIR" && zip -rq "$ZIP" .)
if [ ! -f "$ZIP" ]; then
  # 无 zip 时退回 tar.gz
  ZIP="$ROOT/dist/libpool-$VER.tar.gz"
  tar -czf "$ZIP" -C "$MODULE_DIR" .
fi

echo "==> [4/4] 完成: $ZIP"
echo "    模块目录: $MODULE_DIR"
