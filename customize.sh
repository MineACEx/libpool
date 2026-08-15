#!/system/bin/sh
# =============================================================================
# LibPool · 库池 — customize.sh（安装脚本）
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
#
# 安全设计（重要）：
#   - 本脚本必须在数秒内返回，绝不阻塞安装流程；
#   - 所有耗时任务（装核心库）一律 nohup 后台执行；
#   - 任何步骤失败都不影响系统启动：禁用本模块即可完全恢复。
#   - 全部过程写入 install.log，方便排查（WebUI「关于 → 日志」可查看）。
# =============================================================================

MODDIR=${0%/*}
export LIBPOOL_DIR="$MODDIR"
LOG="$MODDIR/install.log"
mkdir -p "$MODDIR/logs" 2>/dev/null
ts() { date '+%Y-%m-%d %H:%M:%S'; }
log() { echo "[$(ts)] $*" >> "$LOG" 2>/dev/null; }

log "==== LibPool 安装开始 ===="

# 防御：个别 Windows 打包器（如 .NET ZipFile）可能把 zip 条目写成反斜杠，
# 设备端 unzip 会解出 webroot\style.css 这类带反斜杠字面名的文件，损坏模块结构。
# 这里在安装时把所有反斜杠命名的目录/文件规整成标准正斜杠路径（无则零开销）。
log "防御检查：规整反斜杠文件名…"
find "$MODDIR" -name '*\\*' 2>/dev/null | while IFS= read -r bad; do
  good=$(printf '%s' "$bad" | tr '\\' '/')
  mkdir -p "${good%/*}" 2>/dev/null
  mv -f "$bad" "$good" 2>/dev/null
done
log "防御检查完成"

# 检测设备架构：aarch64/arm64 用原生二进制，其余（armv7/x86/x86_64）用 arm 二进制或 shell 兜底
case "$(uname -m)" in
  aarch64|arm64) ARCH_BIN="libman" ;;
  armv7l|armv7|armhf|arm) ARCH_BIN="libman-arm" ;;
  *) ARCH_BIN="" ;; # 其它架构一律用 shell 兜底
esac
log "设备架构：$(uname -m)（bin=$ARCH_BIN）"

# 初始化目录结构
mkdir -p "$MODDIR/libs" 2>/dev/null
mkdir -p "$MODDIR/tools" 2>/dev/null
mkdir -p "$MODDIR/webroot/data" 2>/dev/null
mkdir -p "$MODDIR/system/bin" 2>/dev/null
mkdir -p "$MODDIR/system/lib" 2>/dev/null

# 公告配置目录（用户可在 .git/url.txt 手动填写公告纯文本网址）
mkdir -p "$MODDIR/.git" 2>/dev/null
if [ ! -f "$MODDIR/.git/url.txt" ]; then
  echo "# 在此填写公告纯文本网址（每行一个，取第一个有效 http(s) 链接）" > "$MODDIR/.git/url.txt"
fi
# 云更新配置（可选：update.txt = 云端版本号纯文本网址；download.txt = 下载页面网址）
if [ ! -f "$MODDIR/.git/update.txt" ]; then
  echo "# 在此填写云端版本号纯文本网址（内容为一个版本号，如 1.1.0）" > "$MODDIR/.git/update.txt"
fi
if [ ! -f "$MODDIR/.git/download.txt" ]; then
  echo "# 在此填写下载/更新页面网址（点击更新弹窗中的链接时跳转默认浏览器）" > "$MODDIR/.git/download.txt"
fi

# 初始化状态文件（若不存在；损坏时重置，避免 libman 崩溃）
if [ ! -s "$MODDIR/webroot/data/state.json" ]; then
  echo '{"libs":{}}' > "$MODDIR/webroot/data/state.json"
  log "已初始化 state.json"
fi

# 标记为 KernelSU/APatch 模块（供 KSU WebUI 使用）
chmod 755 "$MODDIR/webroot" 2>/dev/null
chmod 755 "$MODDIR" 2>/dev/null

# 设置可执行权限
chmod 755 "$MODDIR/tools"/* 2>/dev/null
chmod 755 "$MODDIR/libs" 2>/dev/null

# 选择可用的管理工具（原生 > arm 原生 > shell 兜底）
BIN=""
for cand in "$MODDIR/tools/$ARCH_BIN" "$MODDIR/tools/libman" "$MODDIR/tools/libman.sh"; do
  if [ -f "$cand" ] && [ -x "$cand" ]; then
    BIN="$cand"
    break
  fi
done

if [ -n "$BIN" ] && [ -x "$BIN" ] && [ "${BIN##*.}" != "sh" ]; then
  # 后台装 core 库，绝不阻塞安装
  LIBPOOL_DIR="$MODDIR" nohup "$BIN" ensure-core >> "$MODDIR/install.log" 2>&1 &
  log "已用 $BIN 后台安装核心库"
elif [ -f "$MODDIR/tools/libman.sh" ]; then
  chmod 755 "$MODDIR/tools/libman.sh"
  # shell 兜底：核心库安装较重，交给首次开机 service.sh 处理，避免拖慢安装
  log "使用 shell 兜底（$ARCH_BIN 缺失）"
else
  log "警告：未找到可用管理工具（tools 目录如下：）"
  ls -l "$MODDIR/tools" >> "$LOG" 2>/dev/null
fi

# 环境诊断：记录 tools 目录详情与各候选工具是否可执行，
# 安装后若 WebUI 提示"管理工具未就绪"，在「关于 → 查看日志」可直接定位根因。
log "==== 安装环境诊断 ===="
log "架构: $(uname -m)  (期望 bin=$ARCH_BIN)"
for c in "$MODDIR/tools/libman" "$MODDIR/tools/libman-arm" "$MODDIR/tools/libman.sh"; do
  if [ -e "$c" ]; then
    log "存在: $c  (可执行: $([ -x "$c" ] && echo 是 || echo 否))"
  else
    log "缺失: $c"
  fi
done
log "最终使用工具: ${BIN:-无}（无则 WebUI 会提示管理工具未就绪）"

# 清理临时文件
rm -f "$MODDIR/update" 2>/dev/null

log "==== LibPool 安装完成 ===="
echo "LibPool 安装完成！"
echo "请重启设备或使用 KernelSU 管理器打开模块 WebUI 进行库管理。"
