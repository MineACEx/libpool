#!/system/bin/sh
# =============================================================================
# LibPool · 库池 — uninstall.sh（卸载）
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
#
# 卸载时释放全部 bind mount 并清理状态；所有命令容错，失败不影响卸载完成。
# =============================================================================

MODDIR=${0%/*}
export LIBPOOL_DIR="$MODDIR"

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

# 释放全部 bind mount 并清空状态（容错：失败也继续）
if [ -n "$BIN" ]; then
  timeout 60 "$BIN" reset >> "$MODDIR/uninstall.log" 2>&1 || true
fi

# 清理日志
rm -f "$MODDIR/service.log" "$MODDIR/install.log" "$MODDIR/uninstall.log" 2>/dev/null

echo "LibPool 已卸载，感谢使用！"
