#!/system/bin/sh
# =============================================================================
# LibPool · 库池 — service.sh（开机服务，late_start）
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
#
# 安全设计（重要）：
#   - 本脚本是"按需运行"：重放挂载 + 后台补装核心库后立即退出，无常驻进程；
#   - 空闲时模块 CPU/内存占用 ≈ 0；
#   - apply 使用 timeout 限制，任何一步失败都只写日志、不影响开机；
#   - 禁用本模块后此脚本不会被调用，系统完全恢复正常。
# =============================================================================

MODDIR=${0%/*}
export LIBPOOL_DIR="$MODDIR"

# 关键目录 PATH
export PATH="$PATH:/system/bin:/system/xbin:/data/adb/ksu/bin:/data/adb/magisk"

# 检测设备架构，选择可用管理工具（原生 > arm 原生 > shell 兜底）
case "$(uname -m)" in
  aarch64|arm64) ARCH_BIN="libman" ;;
  armv7l|armv7|armhf|arm) ARCH_BIN="libman-arm" ;;
  *) ARCH_BIN="" ;;
esac
BIN=""
for cand in "$MODDIR/tools/$ARCH_BIN" "$MODDIR/tools/libman" "$MODDIR/tools/libman.sh"; do
  if [ -f "$cand" ] && [ -x "$cand" ]; then
    BIN="$cand"
    break
  fi
done

# 等待系统就绪（关键目录可用）。给 apply 全程加 timeout，绝不阻塞开机。
sleep 5

# 1. 重放挂载（本地 bind mount，无网络；timeout 60 兜底）
if [ -n "$BIN" ]; then
  timeout 60 "$BIN" apply >> "$MODDIR/service.log" 2>&1 || \
    echo "LibPool: apply 超时或失败" >> "$MODDIR/service.log"
  chmod 644 "$MODDIR/webroot/data/state.json" 2>/dev/null
fi

# 2. 补装缺失的核心库（后台执行，不阻塞开机；可能联网故后台）
if [ -n "$BIN" ] && [ "${BIN##*.}" != "sh" ]; then
  nohup timeout 600 "$BIN" ensure-core >> "$MODDIR/service.log" 2>&1 &
fi

echo "LibPool service 完成" >> "$MODDIR/service.log"
