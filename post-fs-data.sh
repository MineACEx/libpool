#!/system/bin/sh
# =============================================================================
# LibPool · 库池 — post-fs-data.sh（早期启动）
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
#
# 安全设计：
#   - 早期阶段 /system 尚未完全可写，这里只确保 magic mount 占位目录存在；
#   - 不做任何 mount / 联网 / 重逻辑，保证绝不影响开机；
#   - 实际挂载由 service.sh（late_start）完成。
# =============================================================================

MODDIR=${0%/*}

mkdir -p "$MODDIR/system/bin" 2>/dev/null
mkdir -p "$MODDIR/system/lib" 2>/dev/null

exit 0
