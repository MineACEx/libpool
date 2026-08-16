#!/system/bin/sh
# =============================================================================
# libman.sh — LibPool 原生工具 libman 的 Shell 兜底实现
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
# =============================================================================
# 说明：
#   libman 是 Rust 编译的原生管理工具（list/install/mount 等）。
#   当系统中缺少 libman 二进制时，由 customize.sh / service.sh / WebUI
#   回退到本 Shell 脚本，提供同样的命令行接口与 JSON 输出格式。
#
# 用法: libman.sh <命令> [参数]
#   命令:
#     list               列出所有库及状态（JSON）
#     status             简要状态（JSON）
#     install <id>       下载并安装扩展库
#     remove <id>        删除已安装的库（含卸载）
#     mount <id>         挂载指定库到 /system/bin、/system/lib
#     unmount <id>       卸载指定库
#     toggle <id>        一键开关（缺库自动安装）
#     apply              按已保存状态重放挂载（开机用）
#     ensure-core        补装缺失的核心库（安装/开机用）
#     reset              卸载全部并清理
#     config [mirror <url>]  读取/设置配置（如 mirror）
#     version            打印版本
#
# 输出约定：
#   - 正常结果输出到 stdout（list/status/config 输出 JSON，其余输出提示文本）
#   - 错误信息输出到 stderr，格式为: libman 错误: <内容>
#   - 所有安装/下载过程的进度提示输出到 stderr（避免污染 JSON 输出）
#
# 兼容性：
#   - 仅使用 POSIX sh 兼容语法（mksh / ash / busybox ash 均可运行）
#   - 不要依赖 bash 特有语法
# =============================================================================

# 模块根目录：优先使用环境变量 LIBPOOL_DIR，否则使用默认安装路径
# 环境变量由 customize.sh / service.sh / WebUI 注入
MODDIR=${LIBPOOL_DIR:-/data/adb/modules/libpool}

# 版本号，与 Rust 版保持一致
VERSION="1.0.0"

# 运行日志（logs/libman.log），配合 WebUI「关于 → 查看日志」排查安装/下载/挂载失败。
# 记录每次命令调用与错误，best-effort，写失败不影响功能。
LMAN_LOG="$MODDIR/logs/libman.log"
mkdir -p "$MODDIR/logs" 2>/dev/null || true
logfile() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] libman.sh: $*" >> "$LMAN_LOG" 2>/dev/null || true
}

# 是否开启 set -e？开启后任意命令失败都会导致脚本退出。
# 我们在脚本内部显式使用 "命令 || true" 来处理"允许失败"的命令，
# 避免一个失败命令导致整个脚本提前退出。
set -e

# -----------------------------------------------------------------------------
# 公共路径定义（所有文件操作均基于 MODDIR）
# -----------------------------------------------------------------------------
REPOS_FILE="$MODDIR/webroot/data/repos.json"   # 仓库列表
STATE_FILE="$MODDIR/webroot/data/state.json"   # 状态（每库是否 mounted）
CONFIG_FILE="$MODDIR/libs/config.json"         # 配置（mirror / arch）
CACHE_DIR="$MODDIR/libs/.cache"                # 下载缓存目录
TMP_ROOT="$MODDIR/libs/.tmp"                   # 安装临时目录

# -----------------------------------------------------------------------------
# 工具函数
# -----------------------------------------------------------------------------

# 输出错误信息到 stderr（格式与 Rust 版一致，便于 WebUI 识别）
err() {
    echo "libman 错误: $*" >&2
    logfile "错误: $*"
}

# 输出日志/进度信息到 stderr（不污染 stdout 的 JSON）
log() {
    echo "$*" >&2
}

# 输出标准 JSON 提示（供需要结构化结果时使用）
json_ok() {
    echo "{\"ok\":true,\"msg\":\"$1\"}"
}

# 检测是否有某个外部命令可用
# 用法: has_cmd <命令名>; 返回 0 表示可用
has_cmd() {
    command -v "$1" >/dev/null 2>&1
}

# -----------------------------------------------------------------------------
# 配置读写（libs/config.json，对应 Rust 版 util::load_config / save_config）
# -----------------------------------------------------------------------------

# 读取配置中某个键的值；不存在或损坏时返回默认值
# 用法: get_config <键> <默认值>
get_config() {
    local key="$1"
    local def="$2"
    local val=""
    if [ -f "$CONFIG_FILE" ]; then
        # 简单解析 JSON: "key": "value"（处理了转义引号与逗号/花括号尾随）
        val=$(sed -n "s/.*\"$key\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" "$CONFIG_FILE" | head -n 1)
    fi
    if [ -z "$val" ]; then
        val="$def"
    fi
    echo "$val"
}

# 写入配置（保存 mirror / arch）
# 用法: set_config <键> <值>
set_config() {
    local key="$1"
    local value="$2"
    local mirror arch
    if [ -f "$CONFIG_FILE" ]; then
        mirror=$(get_config "mirror" "https://packages.termux.dev/apt/termux-main")
        arch=$(get_config "arch" "aarch64")
    else
        mirror="https://packages.termux.dev/apt/termux-main"
        arch="aarch64"
    fi
    if [ "$key" = "mirror" ]; then
        mirror="$value"
    fi
    if [ "$key" = "arch" ]; then
        arch="$value"
    fi
    # 生成格式化的 JSON 配置（与 Rust 版 to_string_pretty 结构一致）
    {
        echo "{"
        echo "  \"mirror\": \"$mirror\","
        echo "  \"arch\": \"$arch\""
        echo "}"
    } > "$CONFIG_FILE" 2>/dev/null || {
        err "写入配置失败: $CONFIG_FILE"
        return 1
    }
    return 0
}

# 确保配置文件存在（不存在则写入默认值）
ensure_config() {
    if [ ! -f "$CONFIG_FILE" ]; then
        mkdir -p "$(dirname "$CONFIG_FILE")" 2>/dev/null || true
        set_config "mirror" "https://packages.termux.dev/apt/termux-main" || true
    fi
}

# -----------------------------------------------------------------------------
# 仓库与状态读写
# -----------------------------------------------------------------------------

# 判断某个库是否已安装（存在 meta.json 即视为已安装，对应 Rust 版 is_installed）
is_installed() {
    [ -f "$MODDIR/libs/$1/meta.json" ]
}

# 读取某个库的 meta.json 中指定字段的值
# 用法: get_meta <id> <字段> <默认值>
get_meta() {
    local id="$1"
    local key="$2"
    local def="$3"
    local file="$MODDIR/libs/$id/meta.json"
    local val=""
    if [ -f "$file" ]; then
        val=$(sed -n "s/.*\"$key\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" "$file" | head -n 1)
    fi
    if [ -z "$val" ]; then
        val="$def"
    fi
    echo "$val"
}

# 写入某个库的 meta.json
# 用法: write_meta <id> <version> <source_type> <bins(逗号分隔)> <libs(逗号分隔)>
write_meta() {
    local id="$1"
    local version="$2"
    local source_type="$3"
    local bins="$4"
    local libs="$5"
    local dir="$MODDIR/libs/$id"
    # 将逗号分隔列表转换为 JSON 字符串数组
    local bins_json="[]"
    local libs_json="[]"
    if [ -n "$bins" ]; then
        bins_json=$(list_to_json_array "$bins")
    fi
    if [ -n "$libs" ]; then
        libs_json=$(list_to_json_array "$libs")
    fi
    {
        echo "{"
        echo "  \"version\": \"$version\","
        echo "  \"bins\": $bins_json,"
        echo "  \"libs\": $libs_json,"
        echo "  \"source_type\": \"$source_type\""
        echo "}"
    } > "$dir/meta.json" 2>/dev/null || {
        err "写入元信息失败: $dir/meta.json"
        return 1
    }
    return 0
}

# 将逗号分隔字符串转为 JSON 数组字符串
# 用法: list_to_json_array <逗号分隔字符串>
list_to_json_array() {
    local input="$1"
    local out="["
    local first=1
    local old_ifs="$IFS"
    IFS=','
    for item in $input; do
        IFS="$old_ifs"
        [ -n "$item" ] || continue
        if [ $first -eq 0 ]; then
            out="$out,"
        fi
        # 转义双引号和反斜杠
        item=$(echo "$item" | sed 's/\\/\\\\/g; s/"/\\"/g')
        out="$out\"$item\""
        first=0
        IFS=','
    done
    IFS="$old_ifs"
    out="$out]"
    echo "$out"
}

# 读取状态文件（webroot/data/state.json）
# 状态格式: {"libs": {"<id>": {"mounted": true/false}}}
# 输出: 若文件存在则原样输出，否则输出默认空状态
read_state() {
    if [ -f "$STATE_FILE" ]; then
        cat "$STATE_FILE" 2>/dev/null || echo '{"libs":{}}'
    else
        echo '{"libs":{}}'
    fi
}

# 写入状态文件
write_state() {
    local content="$1"
    mkdir -p "$(dirname "$STATE_FILE")" 2>/dev/null || true
    echo "$content" > "$STATE_FILE" 2>/dev/null || {
        err "写入状态失败: $STATE_FILE"
        return 1
    }
    return 0
}

# 判断某个库在状态中是否标记为已挂载（对应 Rust 版 is_mounted）
# 用法: is_mounted <id>; 返回 0 表示已挂载
is_mounted() {
    local id="$1"
    local pairs
    pairs=$(state_pairs)
    local p
    local old_ifs="$IFS"
    IFS=' '
    for p in $pairs; do
        IFS="$old_ifs"
        if [ "${p%%=*}" = "$id" ]; then
            if [ "${p#*=}" = "true" ]; then
                IFS="$old_ifs"
                return 0
            fi
            IFS="$old_ifs"
            return 1
        fi
        IFS=' '
    done
    IFS="$old_ifs"
    return 1
}

# 解析 state.json，输出 "id=mounted" 对列表（空格分隔）
# 例: "python3=true nodejs=false"
# 用法: state_pairs
state_pairs() {
    local state
    state=$(read_state)
    local pairs=""
    local current=""
    local in_libs=0
    local old_ifs="$IFS"
    IFS='
'
    for line in $state; do
        case "$line" in
            *'"libs"'*) in_libs=1 ;;
            *'"mounted"'*)
                local mv="false"
                if echo "$line" | grep -qE '"mounted"[[:space:]]*:[[:space:]]*true'; then
                    mv="true"
                fi
                if [ -n "$current" ]; then
                    pairs="$pairs $current=$mv"
                fi
                ;;
            *)
                if [ $in_libs -eq 1 ]; then
                    # 匹配形如 "id": { 的行，提取 id
                    if echo "$line" | grep -qE '"[^"]+"[[:space:]]*:[[:space:]]*\{'; then
                        current=$(echo "$line" | sed -n 's/.*"\([^"]*\)"[[:space:]]*:[[:space:]]*{.*/\1/p' | head -n 1)
                    fi
                fi
                ;;
        esac
    done
    IFS="$old_ifs"
    echo "$pairs"
}

# 标记某个库的挂载状态并写回 state.json（对应 Rust 版 mark_mounted）
# 用法: mark_mounted <id> <true|false>
mark_mounted() {
    local id="$1"
    local mounted="$2"
    local pairs new_pairs=""
    pairs=$(state_pairs)
    local found=0
    local p pid pm
    local old_ifs="$IFS"
    IFS=' '
    for p in $pairs; do
        IFS="$old_ifs"
        [ -n "$p" ] || continue
        pid="${p%%=*}"
        pm="${p#*=}"
        if [ "$pid" = "$id" ]; then
            new_pairs="${new_pairs:+$new_pairs }$id=$mounted"
            found=1
        else
            new_pairs="${new_pairs:+$new_pairs }$pid=$pm"
        fi
        IFS=' '
    done
    IFS="$old_ifs"
    if [ $found -eq 0 ]; then
        new_pairs="${new_pairs:+$new_pairs }$id=$mounted"
    fi
    # 生成新的 state.json
    {
        echo "{"
        echo "  \"libs\": {"
        local first=1
        IFS=' '
        for p in $new_pairs; do
            IFS="$old_ifs"
            [ -n "$p" ] || continue
            pid="${p%%=*}"
            pm="${p#*=}"
            if [ $first -eq 0 ]; then
                echo "    },"
            fi
            echo "    \"$pid\": {"
            echo "      \"mounted\": $pm"
            first=0
            IFS=' '
        done
        IFS="$old_ifs"
        if [ $first -eq 0 ]; then
            echo "    }"
        fi
        echo "  }"
        echo "}"
    } > "$STATE_FILE.tmp" 2>/dev/null || return 1
    mv -f "$STATE_FILE.tmp" "$STATE_FILE" 2>/dev/null || return 1
    return 0
}

# 获取所有已标记为 mounted 的库 id（空格分隔）
# 用法: mounted_ids
mounted_ids() {
    local pairs ids=""
    pairs=$(state_pairs)
    local p
    local old_ifs="$IFS"
    IFS=' '
    for p in $pairs; do
        IFS="$old_ifs"
        [ -n "$p" ] || continue
        if [ "${p#*=}" = "true" ]; then
            ids="$ids ${p%%=*}"
        fi
        IFS=' '
    done
    IFS="$old_ifs"
    echo "$ids"
}

# -----------------------------------------------------------------------------
# 下载工具（优先 curl，退回 busybox wget / 系统 wget）
# 对应 Rust 版 util::download
# -----------------------------------------------------------------------------
# 用法: download <url> <目标路径>; 成功返回 0
download() {
    local url="$1"
    local dest="$2"
    mkdir -p "$(dirname "$dest")" 2>/dev/null || true
    if has_cmd curl; then
        curl -fsSL --connect-timeout 15 --retry 2 -o "$dest" "$url" 2>/dev/null && [ -s "$dest" ] && return 0
    fi
    if has_cmd wget; then
        wget -q -T 20 -O "$dest" "$url" 2>/dev/null && [ -s "$dest" ] && return 0
    fi
    if has_cmd busybox; then
        busybox wget -q -T 20 -O "$dest" "$url" 2>/dev/null && [ -s "$dest" ] && return 0
    fi
    rm -f "$dest" 2>/dev/null || true
    return 1
}

# xz 解压：src 路径 -> 输出文件 out
# 对应 Rust 版 util::xz_decompress
# 用法: xz_decompress <src> <out>; 成功返回 0
xz_decompress() {
    local src="$1"
    local out="$2"
    if has_cmd xz; then
        xz -dc "$src" > "$out" 2>/dev/null && return 0
    fi
    if has_cmd unxz; then
        unxz -c "$src" > "$out" 2>/dev/null && return 0
    fi
    if has_cmd busybox; then
        busybox unxz -c "$src" > "$out" 2>/dev/null && return 0
    fi
    return 1
}

# -----------------------------------------------------------------------------
# 挂载 / 卸载（对应 Rust 版 mount.rs）
# -----------------------------------------------------------------------------

# 绑定挂载单个文件: 源文件 -> 目标文件（幂等）
# 对应 Rust 版 bind_mount
# 用法: bind_mount <源> <目标>
bind_mount() {
    local src="$1"
    local dst="$2"
    # 确保目标父目录存在
    mkdir -p "$(dirname "$dst")" 2>/dev/null || true
    # 若目标已是 mount 点，先 umount 再重新 bind
    if mount | grep -q " on $dst "; then
        umount "$dst" 2>/dev/null || true
    fi
    # 源文件必须存在
    if [ ! -e "$src" ]; then
        err "源文件不存在: $src"
        return 1
    fi
    # 若目标不存在，touch 创建占位（失败不致命，部分 KSU 内核允许直接 bind）
    if [ ! -e "$dst" ]; then
        touch "$dst" 2>/dev/null || true
    fi
    # 执行 bind mount；失败时尝试 noexec 降级参数
    if ! mount --bind "$src" "$dst" 2>/dev/null; then
        if ! mount -o bind,noexec "$src" "$dst" 2>/dev/null; then
            err "bind mount 失败: $src -> $dst"
            return 1
        fi
    fi
    return 0
}

# 挂载指定库：将 libs/<id>/bin/* 绑定到 /system/bin/，
#          将 libs/<id>/lib/* 绑定到 /system/lib/（lib 失败不致命）
# 安全红线：若系统里已有同名 bin/so，一律跳过、绝不覆盖（对应 Rust 版 copy_if_safe）。
# 对应 Rust 版 mount_lib
# 用法: mount_lib <id>
mount_lib() {
    local id="$1"
    local lib_root="$MODDIR/libs/$id"
    local bin_dir="$lib_root/bin"
    local lib_dir="$lib_root/lib"
    local name
    # 挂载 bin 目录下所有条目
    if [ -d "$bin_dir" ]; then
        local old_ifs="$IFS"
        IFS='
'
        for name in $(ls -A "$bin_dir" 2>/dev/null | sort); do
            IFS="$old_ifs"
            [ -n "$name" ] || continue
            if [ -e "/system/bin/$name" ]; then
                logfile "$id: 跳过 $name（/system/bin/$name 已存在，不覆盖）"
                IFS='
'
                continue
            fi
            bind_mount "$bin_dir/$name" "/system/bin/$name" || true
            IFS='
'
        done
        IFS="$old_ifs"
    fi
    # 挂载 lib 目录下所有条目（失败不致命；系统已有同名 so 一律跳过，绝不覆盖）
    if [ -d "$lib_dir" ]; then
        local old_ifs2="$IFS"
        IFS='
'
        for name in $(ls -A "$lib_dir" 2>/dev/null | sort); do
            IFS="$old_ifs2"
            [ -n "$name" ] || continue
            if [ -e "/system/lib/$name" ]; then
                logfile "$id: 跳过 $name（/system/lib/$name 已存在，不覆盖）"
                IFS='
'
                continue
            fi
            bind_mount "$lib_dir/$name" "/system/lib/$name" 2>/dev/null || true
            IFS='
'
        done
        IFS="$old_ifs2"
    fi
    echo "已挂载: $id"
    return 0
}

# 卸载指定库的所有 bind mount（读取 meta.json 的 bins / libs）
# 对应 Rust 版 unmount_lib_by_id
# 用法: unmount_lib <id>
unmount_lib() {
    local id="$1"
    local meta_path="$MODDIR/libs/$id/meta.json"
    if [ ! -f "$meta_path" ]; then
        return 0  # 未安装则视为已清理
    fi
    local bins libs
    bins=$(get_meta "$id" "bins" "")
    libs=$(get_meta "$id" "libs" "")
    # 卸载 bin
    local old_ifs="$IFS"
    IFS=','
    for b in $bins; do
        IFS="$old_ifs"
        [ -n "$b" ] || continue
        umount "/system/bin/$b" 2>/dev/null || true
        IFS=','
    done
    IFS="$old_ifs"
    # 卸载 lib
    old_ifs="$IFS"
    IFS=','
    for l in $libs; do
        IFS="$old_ifs"
        [ -n "$l" ] || continue
        umount "/system/lib/$l" 2>/dev/null || true
        IFS=','
    done
    IFS="$old_ifs"
    echo "已卸载: $id"
    return 0
}

# 卸载所有已挂载的库
# 对应 Rust 版 unmount_all
# 用法: unmount_all
unmount_all() {
    local ids
    ids=$(mounted_ids)
    local old_ifs="$IFS"
    IFS=' '
    for id in $ids; do
        IFS="$old_ifs"
        [ -n "$id" ] || continue
        unmount_lib "$id" 2>/dev/null || true
        mark_mounted "$id" false 2>/dev/null || true
        IFS=' '
    done
    IFS="$old_ifs"
    return 0
}

# -----------------------------------------------------------------------------
# 安装逻辑
# -----------------------------------------------------------------------------

# 从 URL 下载并安装（type=url）
# 对应 Rust 版 install_from_url
# 用法: install_from_url <tmp目录> <entry_json字段...>
# 需要用到 entry 的 url / archive / bins / libs / version（通过 get_entry 传入）
install_from_url() {
    local tmp="$1"
    local url="$2"
    local archive="$3"
    local bins="$4"
    local libs="$5"
    local version="$6"
    local file="$tmp/download"

    [ -n "$archive" ] || archive="raw"
    log "下载 $url …"
    if ! download "$url" "$file"; then
        err "下载失败: $url"
        return 1
    fi

    case "$archive" in
        raw)
            # 直接当作二进制放入 usr/bin
            mkdir -p "$tmp/usr/bin" 2>/dev/null || true
            local fname
            fname=$(first_of_list "$bins")
            [ -n "$fname" ] || fname="app"
            cp "$file" "$tmp/usr/bin/$fname" 2>/dev/null || {
                err "复制二进制失败"
                return 1
            }
            ;;
        zip)
            if ! ( cd "$tmp" && unzip -o -q "$file" ); then
                err "解压 zip 失败"
                return 1
            fi
            ;;
        tar.gz|tgz)
            if ! ( cd "$tmp" && tar -xzf "$file" ); then
                err "解压 tar.gz 失败"
                return 1
            fi
            ;;
        tar.xz)
            if ! ( cd "$tmp" && tar -xJf "$file" ); then
                err "解压 tar.xz 失败"
                return 1
            fi
            ;;
        *)
            err "不支持的归档类型: $archive"
            return 1
            ;;
    esac
    return 0
}

# 返回逗号分隔列表的第一个元素
# 用法: first_of_list <逗号分隔列表>
first_of_list() {
    local list="$1"
    local old_ifs="$IFS"
    IFS=','
    for item in $list; do
        IFS="$old_ifs"
        echo "$item"
        return 0
    done
    IFS="$old_ifs"
}

# 从 Termux 仓库下载 .deb 并解包（type=termux / bundled）
# 对应 Rust 版 install_from_termux
# 用法: install_from_termux <tmp目录> <pkg>
install_from_termux() {
    local tmp="$1"
    local pkg="$2"
    local mirror arch
    mirror=$(get_config "mirror" "https://packages.termux.dev/apt/termux-main")
    arch=$(get_config "arch" "aarch64")
    mirror=$(echo "$mirror" | sed 's:/*$::')  # 去掉末尾斜杠

    mkdir -p "$CACHE_DIR" 2>/dev/null || true
    local idx_xz="$CACHE_DIR/Packages.$arch.xz"
    local idx_txt="$CACHE_DIR/Packages.$arch"
    local idx_url="$mirror/dists/stable/main/binary-$arch/Packages.xz"

    # 1) 下载 Packages 索引（失败退回未压缩版）
    log "下载仓库索引…"
    if ! download "$idx_url" "$idx_xz"; then
        local alt="$mirror/dists/stable/main/binary-$arch/Packages"
        if ! download "$alt" "$idx_txt"; then
            err "下载仓库索引失败"
            return 1
        fi
    elif [ -s "$idx_xz" ]; then
        # 2) xz 解压索引
        if ! xz_decompress "$idx_xz" "$idx_txt"; then
            err "索引解压失败"
            return 1
        fi
    fi

    # 3) 解析出该包的 Version 与 Filename
    local version filename
    if ! parse_packages "$idx_txt" "$pkg" version filename; then
        err "仓库索引中找不到包: $pkg"
        return 1
    fi
    log "$pkg -> $version ($filename)"

    # 4) 下载 .deb
    local deb="$CACHE_DIR/$pkg.deb"
    local deb_url="$mirror/$filename"
    if ! download "$deb_url" "$deb"; then
        err "下载 .deb 失败: $deb_url"
        return 1
    fi
    logfile "已下载 .deb: $pkg（$(wc -c < "$deb" 2>/dev/null) 字节）"

    # 5) 从 .deb（ar 归档）提取 data 归档（支持 xz / gz / zst）并解包
    local data_arch="" data_ext="" d
    for ext in xz gz zst; do
        d="$tmp/data.tar.$ext"
        if extract_deb_member "$deb" "data.tar.$ext" "$d" 2>/dev/null && [ -s "$d" ]; then
            data_arch="$d"
            data_ext="$ext"
            break
        fi
    done
    if [ -z "$data_arch" ]; then
        err "无法从 .deb 提取 data 归档（xz/gz/zst 均未找到）"
        return 1
    fi
    logfile "已提取 data.tar.$data_ext（$(wc -c < "$data_arch" 2>/dev/null) 字节）"
    case "$data_ext" in
        xz)
            if ! xz_decompress "$data_arch" "$tmp/data.tar"; then
                err "解压 data.tar.xz 失败"
                return 1
            fi
            # 6) 解压 data.tar 到 tmp 根目录（去掉 ./ 前缀）
            if ! ( cd "$tmp" && tar -xf "$tmp/data.tar" 2>/dev/null ); then
                if ! ( cd "$tmp" && busybox tar -xf "$tmp/data.tar" 2>/dev/null ); then
                    err "解包 data.tar 失败"
                    return 1
                fi
            fi
            ;;
        gz)
            if ! ( cd "$tmp" && tar -xzf "$data_arch" 2>/dev/null ); then
                if ! ( cd "$tmp" && busybox tar -xzf "$data_arch" 2>/dev/null ); then
                    err "解包 data.tar.gz 失败"
                    return 1
                fi
            fi
            ;;
        zst)
            # zstd：优先让 tar 自动识别；否则 zstd/unzstd 解压后再 tar（尽力而为）
            if ! ( cd "$tmp" && tar -xf "$data_arch" 2>/dev/null ); then
                if has_cmd zstd; then
                    zstd -dc "$data_arch" > "$tmp/data.tar" 2>/dev/null || true
                elif has_cmd unzstd; then
                    unzstd -dc "$data_arch" > "$tmp/data.tar" 2>/dev/null || true
                fi
                if [ ! -s "$tmp/data.tar" ] || ! ( cd "$tmp" && tar -xf "$tmp/data.tar" 2>/dev/null ); then
                    if ! ( cd "$tmp" && busybox tar -xf "$data_arch" 2>/dev/null ); then
                        err "解包 data.tar.zst 失败"
                        return 1
                    fi
                fi
            fi
            ;;
    esac
    return 0
}

# 从 .deb（ar 归档）中提取指定成员到目标文件
# .deb 本质是 ar 归档，成员名形如 data.tar.xz（可能带尾随斜杠）
# 用法: extract_deb_member <deb文件> <成员名> <输出文件>
extract_deb_member() {
    local deb="$1"
    local member="$2"
    local out="$3"
    # 使用 ar 命令（若系统有）
    if has_cmd ar; then
        local dir
        dir=$(mktemp -d 2>/dev/null || echo "$TMP_ROOT/deb_$$")
        mkdir -p "$dir" 2>/dev/null || true
        if ( cd "$dir" && ar p "$deb" "$member" > "$out" 2>/dev/null ) && [ -s "$out" ]; then
            rm -rf "$dir" 2>/dev/null || true
            return 0
        fi
        rm -rf "$dir" 2>/dev/null || true
    fi
    # 无 ar 命令：手工解析 ar 归档（魔数 !<arch>\n，头 60 字节）
    # 仅支持常规短文件名成员；长名表（//）场景保守跳过
    local size name
    if [ ! -f "$deb" ]; then
        return 1
    fi
    # 读取魔数
    if ! dd if="$deb" bs=1 count=8 2>/dev/null | grep -q '^!<arch>$'; then
        return 1
    fi
    local offset=8
    local filesize
    filesize=$(wc -c < "$deb")
    while [ "$offset" -lt "$filesize" ]; do
        # 读取 60 字节头部
        local header
        header=$(dd if="$deb" bs=1 skip=$offset count=60 2>/dev/null)
        [ -n "$header" ] || break
        name=$(echo "$header" | cut -c1-16 | tr -d ' ' | sed 's:/$::')
        # 大小字段在第 48-57 字符（0-based 48..57）
        size=$(echo "$header" | cut -c49-58 | tr -d ' ')
        size=$(echo "$size" | sed 's/[^0-9]//g')
        [ -n "$size" ] || size=0
        # 校验尾部魔数 "`\n"（单引号包裹反引号，避免命令替换）
        # 注意：$(...) 命令替换会剥掉尾部换行，故 tailmagic 为单个反引号字符
        local tailmagic
        tailmagic=$(echo "$header" | cut -c59-60)
        local data_start=$((offset + 60))
        if [ "$tailmagic" = '`' ]; then
            # 非标准头（如 GNU 长名表），跳过该成员数据
            offset=$((data_start + size))
            [ $((size % 2)) -eq 1 ] && offset=$((offset + 1))
            continue
        fi
        if [ "$name" = "$member" ]; then
            dd if="$deb" bs=1 skip=$data_start count=$size 2>/dev/null > "$out"
            [ -s "$out" ] && return 0
            return 1
        fi
        offset=$((data_start + size))
        [ $((size % 2)) -eq 1 ] && offset=$((offset + 1))
    done
    return 1
}

# 解析 Packages 文本：找出指定包的 Version 与 Filename
# 用法: parse_packages <Packages文件> <pkg> <version变量名> <filename变量名>
parse_packages() {
    local file="$1"
    local pkg="$2"
    local cur_pkg=""
    local version=""
    local filename=""
    local found=0
    local old_ifs="$IFS"
    IFS='
'
    while IFS= read -r line; do
        case "$line" in
            "")
                # 段落结束：若已找到目标包则返回
                if [ $found -eq 1 ]; then
                    if [ -z "$filename" ]; then
                        IFS="$old_ifs"
                        return 1  # 缺少 Filename
                    fi
                    # 通过 eval 间接赋值给调用者变量
                    eval "$3='$version'"
                    eval "$4='$filename'"
                    IFS="$old_ifs"
                    return 0
                fi
                cur_pkg=""
                version=""
                filename=""
                ;;
            Package:*)
                cur_pkg=$(echo "$line" | cut -d' ' -f2- | tr -d ' ')
                if [ "$cur_pkg" = "$pkg" ]; then
                    found=1
                else
                    found=0
                fi
                ;;
            Version:*)
                if [ $found -eq 1 ]; then
                    version=$(echo "$line" | cut -d' ' -f2- | tr -d ' ')
                fi
                ;;
            Filename:*)
                if [ $found -eq 1 ]; then
                    filename=$(echo "$line" | cut -d' ' -f2- | tr -d ' ')
                fi
                ;;
        esac
    done < "$file"
    IFS="$old_ifs"
    if [ $found -eq 1 ]; then
        if [ -z "$filename" ]; then
            return 1
        fi
        eval "$3='$version'"
        eval "$4='$filename'"
        return 0
    fi
    return 1
}

# 从 repos.json 中读取某个库的字段值
# repos.json 是 JSON 数组，格式为每个对象一行一个字段（pretty 打印）：
#   { "id": "python3", "name": "...", ... }
# 用法: get_entry <id> <字段>; 输出该字段值（字符串）。未找到输出空。
get_entry() {
    local id="$1"
    local field="$2"
    if [ ! -f "$REPOS_FILE" ]; then
        echo ""
        return 0
    fi
    # 使用 awk 做对象级解析：以花括号深度区分对象边界，
    # 先找到 id 匹配的对象，再在该对象内提取目标字段。
    # 注意：不使用环境变量前缀（gawk/busybox/toybox 兼容性），直接传参。
    awk -v tid="$id" -v tfield="$field" '
    function trim(s){ gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s }
    # 提取字符串 s 中第一个 "..." 引号内的内容
    function qstr(s,   a, b) {
        a = index(s, "\"")
        if (a == 0) return ""
        b = index(substr(s, a+1), "\"")
        if (b == 0) return ""
        return substr(s, a+1, b-1)
    }
    BEGIN{ depth=0; matched=0 }
    {
        # 1) 在对象内（depth>=1）且尚未匹配时，检测 id 字段
        if (!matched && depth >= 1) {
            if (match($0, /"[A-Za-z0-9_.+-]+"[ \t]*:[ \t]*"/)) {
                if (qstr($0) == "id") {
                    # 值的开引号已被正则消费，剩余形如 "toolx",（值内容 + 尾随）
                    rest = substr($0, RSTART+RLENGTH)
                    vq = index(rest, "\"")
                    if (vq > 0 && substr(rest, 1, vq-1) == tid) matched = 1
                }
            }
        }
        # 2) 在匹配对象内，提取目标字段并输出
        if (matched) {
            if (match($0, /"[A-Za-z0-9_.+-]+"[ \t]*:/)) {
                if (qstr($0) == tfield) {
                    v = trim(substr($0, RSTART+RLENGTH))
                    gsub(/,$/, "", v)
                    # 字符串值去引号；数组/布尔/数字保持原样
                    gsub(/^"|"$/, "", v)
                    print v
                    exit
                }
            }
        }
        # 3) 更新花括号深度，用于识别对象边界
        line = $0; opens = gsub(/{/, "{", line)
        line = $0; closes = gsub(/}/, "}", line)
        depth += opens - closes
        if (depth < 0) depth = 0
        if (depth == 0) matched = 0
    }
    ' "$REPOS_FILE"
}

# 判断 repos.json 中是否存在指定库
# 用法: repo_has <id>; 返回 0 存在
repo_has() {
    local id="$1"
    local val
    val=$(get_entry "$id" "id")
    [ "$val" = "$id" ]
}

# 从 repos.json 中读取某个库的 bins 列表（JSON 数组 -> 逗号分隔）
# 用法: get_entry_bins <id>; 输出逗号分隔的 bin 文件名
get_entry_bins() {
    local id="$1"
    local raw
    raw=$(get_entry "$id" "bins")
    if [ -z "$raw" ] || [ "$raw" = "[]" ] || [ "$raw" = '""' ]; then
        echo ""
        return 0
    fi
    # 将 JSON 数组 ["a","b"] 转为 a,b
    echo "$raw" | sed 's/\[//g; s/\]//g; s/","/,/g; s/"//g'
}

# 从 repos.json 中读取某个库的 libs 列表（JSON 数组 -> 逗号分隔）
get_entry_libs() {
    local id="$1"
    get_entry_bins "$id"  # 复用同一解析逻辑
}

# -----------------------------------------------------------------------------
# 主安装流程（对应 Rust 版 install_lib）
# 用法: install_lib <id> <verbose>
install_lib() {
    local id="$1"
    local verbose="$2"
    local lib_root="$MODDIR/libs/$id"

    # 已安装则提示（verbose 模式下）
    if is_installed "$id"; then
        # 自愈：若上次只写了空壳（0 bin 0 lib），说明那次安装失败，删掉重装
        local ob ol
        ob=$(get_meta "$id" "bins" "")
        ol=$(get_meta "$id" "libs" "")
        if [ -z "$ob" ] && [ -z "$ol" ]; then
            logfile "检测到 $id 安装为空壳（无 bin/lib），删除并重装"
            rm -rf "$MODDIR/libs/$id" 2>/dev/null || true
        else
            if [ "$verbose" = "1" ]; then
                echo "已安装过: $id，可先 remove 再重装"
            fi
            return 0
        fi
    fi

    # 读取仓库条目
    local etype epub url archive bins libs version
    etype=$(get_entry "$id" "type")
    [ -n "$etype" ] || etype="termux"
    epub=$(get_entry "$id" "pkg")
    url=$(get_entry "$id" "url")
    archive=$(get_entry "$id" "archive")
    bins=$(get_entry_bins "$id")
    libs=$(get_entry_libs "$id")
    version=$(get_entry "$id" "version")

    # 创建目录与临时目录
    mkdir -p "$lib_root" 2>/dev/null || true
    local tmp="$TMP_ROOT/$id"
    rm -rf "$tmp" 2>/dev/null || true
    mkdir -p "$tmp" 2>/dev/null || {
        err "创建临时目录失败: $tmp"
        return 1
    }

    # 按类型安装
    local meta_version=""
    local meta_source=""
    case "$etype" in
        termux|bundled)
            local pkg
            if [ -n "$epub" ]; then
                pkg="$epub"
            else
                pkg="$id"
            fi
            if ! install_from_termux "$tmp" "$pkg"; then
                rm -rf "$tmp" 2>/dev/null || true
                return 1
            fi
            meta_version="$version"
            meta_source="termux"
            ;;
        url)
            if ! install_from_url "$tmp" "$url" "$archive" "$bins" "$libs" "$version"; then
                rm -rf "$tmp" 2>/dev/null || true
                return 1
            fi
            meta_version="$version"
            meta_source="url"
            ;;
        *)
            err "未知安装类型: $etype"
            rm -rf "$tmp" 2>/dev/null || true
            return 1
            ;;
    esac

    # 定位真正的 usr 根目录（Termux 的 .deb 解包后带前缀
    # data/data/com.termux/files/usr，标准 deb 则是 usr）
    local usr="$tmp/usr"
    if [ ! -d "$usr" ] && [ -d "$tmp/data/data/com.termux/files/usr" ]; then
        usr="$tmp/data/data/com.termux/files/usr"
    fi

    # 移动 bin / lib 到目标
    local bin_src="$usr/bin"
    local lib_src="$usr/lib"
    local bin_dst="$lib_root/bin"
    local lib_dst="$lib_root/lib"
    mkdir -p "$bin_dst" 2>/dev/null || true
    mkdir -p "$lib_dst" 2>/dev/null || true

    local installed_bins=""
    local installed_libs=""
    local name

    if [ -d "$bin_src" ]; then
        local old_ifs="$IFS"
        IFS='
'
        for name in $(ls -A "$bin_src" 2>/dev/null | sort); do
            IFS="$old_ifs"
            [ -n "$name" ] || continue
            # 复制普通文件与符号链接（跟随链接复制）
            if [ -f "$bin_src/$name" ] || [ -L "$bin_src/$name" ]; then
                cp -f "$bin_src/$name" "$bin_dst/$name" 2>/dev/null || true
                installed_bins="${installed_bins:+$installed_bins,}$name"
            fi
            IFS='
'
        done
        IFS="$old_ifs"
    fi

    if [ -d "$lib_src" ]; then
        local old_ifs2="$IFS"
        IFS='
'
        for name in $(ls -A "$lib_src" 2>/dev/null | sort); do
            IFS="$old_ifs2"
            [ -n "$name" ] || continue
            # 只复制 .so 文件
            case "$name" in
                *.so|*.so.*)
                    cp -f "$lib_src/$name" "$lib_dst/$name" 2>/dev/null || true
                    installed_libs="${installed_libs:+$installed_libs,}$name"
                    ;;
            esac
            IFS='
'
        done
        IFS="$old_ifs2"
    fi

    # 合并 bins：安装产物 + 仓库声明的 bins（去重）
    local final_bins="$installed_bins"
    local old_ifs3="$IFS"
    IFS=','
    for b in $bins; do
        IFS="$old_ifs3"
        [ -n "$b" ] || continue
        if ! echo "$final_bins" | grep -qE "(^|,)$b(,|$)"; then
            final_bins="${final_bins:+$final_bins,}$b"
        fi
        IFS=','
    done
    IFS="$old_ifs3"

    # 设置可执行权限
    IFS=','
    for b in $final_bins; do
        IFS="$old_ifs3"
        [ -n "$b" ] || continue
        chmod 755 "$bin_dst/$b" 2>/dev/null || true
        IFS=','
    done
    IFS="$old_ifs3"

    # 写 meta.json
    write_meta "$id" "$meta_version" "$meta_source" "$final_bins" "$installed_libs" || {
        rm -rf "$tmp" 2>/dev/null || true
        return 1
    }

    # 记录安装结果（供日志排查：bin/lib 是否为空；空则列出 tmp 内容定位解压问题）
    logfile "install_lib($id): bins=[$final_bins] libs=[$installed_libs]"
    if [ -z "$final_bins" ] && [ -z "$installed_libs" ]; then
        logfile "警告: $id 未装出任何文件，tmp 目录结构:"
        ls -la "$tmp" >> "$LMAN_LOG" 2>/dev/null || true
        [ -d "$tmp/data" ] && ls -laR "$tmp/data" >> "$LMAN_LOG" 2>/dev/null || true
    fi

    # 清理临时目录
    rm -rf "$tmp" 2>/dev/null || true

    if [ "$verbose" = "1" ]; then
        local bin_count lib_count
        bin_count=$(count_list "$final_bins")
        lib_count=$(count_list "$installed_libs")
        echo "安装完成: $id v$meta_version（$bin_count 个可执行文件, $lib_count 个动态库）"
    fi
    return 0
}

# 统计逗号分隔列表的元素个数
# 用法: count_list <逗号分隔列表>
count_list() {
    local list="$1"
    if [ -z "$list" ]; then
        echo "0"
        return 0
    fi
    local old_ifs="$IFS"
    IFS=','
    local n=0
    for _ in $list; do
        n=$((n + 1))
    done
    IFS="$old_ifs"
    echo "$n"
}

# -----------------------------------------------------------------------------
# 各子命令实现
# -----------------------------------------------------------------------------

# list：合并仓库与本地状态，输出完整 JSON 数组
# 对应 Rust 版 cmd_list
cmd_list() {
    if [ ! -f "$REPOS_FILE" ]; then
        err "读取仓库列表失败: $REPOS_FILE 不存在"
        return 1
    fi
    # 预计算已安装信息: id=version 逗号分隔（用于传给 awk 判断 installed/version）
    local installed_map=""
    local ids id ver
    ids=$(installed_ids)
    local old_ifs="$IFS"
    IFS=' '
    for id in $ids; do
        IFS="$old_ifs"
        [ -n "$id" ] || continue
        ver=$(get_meta "$id" "version" "")
        installed_map="${installed_map:+$installed_map,}$id=$ver"
        IFS=' '
    done
    IFS="$old_ifs"
    local state
    state=$(read_state)
    # 用 awk 逐条解析 repos.json 并合并本地状态输出 JSON
    # 注意：使用 -v 显式传参（而非环境变量前缀），保证 gawk / busybox awk / toybox awk 兼容
    awk -v INSTALLED_MAP="$installed_map" -v STATE_JSON="$state" '
    function trim(s){ gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s }
    function unq(s){ gsub(/^"|"$/, "", s); return s }
    function esc(s){
        gsub(/\\/, "\\\\", s)
        gsub(/"/, "\\\"", s)
        gsub(/\n/, "\\n", s)
        return s
    }
    BEGIN{
        # 解析 STATE_JSON：记录每个 id 的 mounted 状态
        split(STATE_JSON, slines, "\n")
        cur=""
        for (i in slines) {
            line=slines[i]
            # 先判断 mounted 字段行，再匹配 "key"（只取引号内的键名）
            if (match(line, /"mounted"[ \t]*:[ \t]*(true|false)/)) {
                mv = substr(line, RSTART, RLENGTH)
                gsub(/.*:[ \t]*/, "", mv)
                m[cur] = (mv=="true") ? "true" : "false"
            } else if (match(line, /"[A-Za-z0-9_-]+"/)) {
                key = unq(substr(line, RSTART, RLENGTH))
                cur = key
            }
        }
        # 解析 INSTALLED_MAP: id=version,...
        np = split(INSTALLED_MAP, parts, ",")
        for (i=1; i<=np; i++) {
            p = parts[i]
            if (p=="") continue
            eq = index(p, "=")
            if (eq > 0) {
                iid = substr(p, 1, eq-1)
                iv  = substr(p, eq+1)
                meta_installed[iid] = 1
                meta_ver[iid] = iv
            }
        }
        first=1
    }
    {
        if (match($0, /"[A-Za-z0-9_-]+"[ \t]*:/)) {
            key = unq(trim(substr($0, RSTART+1, RLENGTH-2)))
            val = trim(substr($0, RSTART+RLENGTH))
            gsub(/,$/, "", val)
            gsub(/^\{/, "", val)
            gsub(/^\[/, "", val)
            gsub(/,$/, "", val)
            gsub(/\}$/, "", val)
            gsub(/\]$/, "", val)
            if (val ~ /^"/) v=unq(trim(val)); else v=trim(val)
            if (key=="id") { cur_id=v }
            fields[cur_id,key]=v
        }
        if ($0 ~ /^[ \t]*}/ && cur_id!="") {
            # 对象结束：输出该条
            id=fields[cur_id,"id"]
            name=fields[cur_id,"name"]
            desc=fields[cur_id,"desc"]
            cat=fields[cur_id,"category"]
            typ=fields[cur_id,"type"]
            core=fields[cur_id,"core"]
            ver=fields[cur_id,"version"]
            if (name=="") name=id
            if (cat=="") cat=""
            if (typ=="") typ="termux"
            if (core!="true") core="false"
            mounted = (m[id]=="true") ? "true" : "false"
            installed = "false"
            if (meta_installed[id]==1) installed="true"
            if (ver=="" && installed=="true") ver = meta_ver[id]
            if (!first) printf ",\n"
            else printf "[\n"
            first=0
            printf "  {\"id\":\"%s\",\"name\":\"%s\",\"desc\":\"%s\",\"category\":\"%s\",\"type\":\"%s\",\"core\":%s,\"installed\":%s,\"mounted\":%s,\"version\":\"%s\"}", esc(id), esc(name), esc(desc), esc(cat), esc(typ), core, installed, mounted, esc(ver)
            cur_id=""
        }
    }
    END{ if (first) printf "[]\n"; else printf "\n]\n" }
    ' "$REPOS_FILE"
}

# 获取所有已安装库的 id（基于 libs 目录下有 meta.json 的目录）
# 用法: installed_ids; 输出空格分隔
installed_ids() {
    local out=""
    local old_ifs="$IFS"
    IFS='
'
    for d in $(ls -A "$MODDIR/libs" 2>/dev/null | sort); do
        IFS="$old_ifs"
        [ -f "$MODDIR/libs/$d/meta.json" ] || continue
        out="$out $d"
        IFS='
'
    done
    IFS="$old_ifs"
    echo "$out"
}

# status：简要 JSON 状态（total/installed/mounted/core_ok/core_total/arch）
# 对应 Rust 版 cmd_status
cmd_status() {
    local total installed mounted core_ok core_total arch
    total=0
    installed=0
    core_ok=0
    core_total=0
    local id etype corev
    # 统计 total 与 core_total：遍历 repos.json
    if [ -f "$REPOS_FILE" ]; then
        # 用 grep 统计顶层对象数量（近似：统计 "id": 出现次数）
        total=$(grep -c '"id"[[:space:]]*:' "$REPOS_FILE" 2>/dev/null || echo 0)
        core_total=$(grep -c '"core"[[:space:]]*:[[:space:]]*true' "$REPOS_FILE" 2>/dev/null || echo 0)
    fi
    # 统计 installed 与 core_ok：遍历 libs 目录
    local ids
    ids=$(installed_ids)
    local old_ifs="$IFS"
    IFS=' '
    for id in $ids; do
        IFS="$old_ifs"
        [ -n "$id" ] || continue
        installed=$((installed + 1))
        # 若该库是核心库则 core_ok +1
        if echo " core_total_marker " >/dev/null; then :; fi
        # 判断是否 core：通过 repos.json 查询
        if [ -f "$REPOS_FILE" ]; then
            if awk -v tid="$id" '
            function trim(s){ gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s }
            function unq(s){ gsub(/^"|"$/, "", s); return s }
            BEGIN{ m=0; c=0 }
            {
                if (match($0, /"[A-Za-z0-9_-]+"[ \t]*:/)) {
                    key=unq(trim(substr($0, RSTART+1, RLENGTH-2)))
                    val=trim(substr($0, RSTART+RLENGTH)); gsub(/,/,"",val)
                    if (key=="id") { if (unq(trim(val))==tid) m=1; else m=0 }
                    if (m && key=="core" && val=="true") c=1
                }
            }
            END{ if (c) exit 0; exit 1 }
            ' "$REPOS_FILE"; then
                core_ok=$((core_ok + 1))
            fi || true
        fi
        IFS=' '
    done
    IFS="$old_ifs"

    # 统计 mounted
    mounted=0
    local mids
    mids=$(mounted_ids)
    for id in $mids; do
        [ -n "$id" ] && mounted=$((mounted + 1))
    done

    arch=$(get_config "arch" "aarch64")
    echo "{\"total\":$total,\"installed\":$installed,\"mounted\":$mounted,\"core_ok\":$core_ok,\"core_total\":$core_total,\"arch\":\"$arch\"}"
    return 0
}

# install <id>：下载并安装扩展库（对应 Rust 版 install_cmd）
cmd_install() {
    local id="$1"
    if [ -z "$id" ]; then
        err "缺少库 id"
        return 1
    fi
    # 检查仓库中是否存在
    if ! repo_has "$id"; then
        err "仓库中不存在库: $id"
        return 1
    fi
    install_lib "$id" 1
}

# remove <id>：先卸载（若有挂载），再删除文件与状态
# 对应 Rust 版 remove_cmd
cmd_remove() {
    local id="$1"
    if [ -z "$id" ]; then
        err "缺少库 id"
        return 1
    fi
    # 先卸载（若已挂载）
    if is_mounted "$id"; then
        unmount_lib "$id" 2>/dev/null || true
    fi
    mark_mounted "$id" false 2>/dev/null || true
    rm -rf "$MODDIR/libs/$id" 2>/dev/null || true
    echo "已删除库: $id"
    return 0
}

# mount <id>：挂载指定库（对应 Rust 版 mount_cmd）
cmd_mount() {
    local id="$1"
    if [ -z "$id" ]; then
        err "缺少库 id"
        return 1
    fi
    if ! is_installed "$id"; then
        err "库未安装: $id，请先下载"
        return 1
    fi
    mount_lib "$id" || return 1
    mark_mounted "$id" true || return 1
    return 0
}

# unmount <id>：卸载指定库（对应 Rust 版 unmount_cmd）
cmd_unmount() {
    local id="$1"
    if [ -z "$id" ]; then
        err "缺少库 id"
        return 1
    fi
    unmount_lib "$id" || return 1
    mark_mounted "$id" false || return 1
    return 0
}

# toggle <id>：一键开关（对应 Rust 版 toggle_cmd）
cmd_toggle() {
    local id="$1"
    if [ -z "$id" ]; then
        err "缺少库 id"
        return 1
    fi
    if is_mounted "$id"; then
        # 已挂载则卸载
        cmd_unmount "$id"
    else
        # 未安装则先安装
        if ! is_installed "$id"; then
            if ! repo_has "$id"; then
                err "仓库中不存在库: $id"
                return 1
            fi
            install_lib "$id" 1 || return 1
        fi
        cmd_mount "$id"
    fi
}

# apply：按已保存状态重放挂载（对应 Rust 版 apply_cmd，开机用）
cmd_apply() {
    local ids
    ids=$(mounted_ids)
    local ok=0 fail=0 id
    local old_ifs="$IFS"
    IFS=' '
    for id in $ids; do
        IFS="$old_ifs"
        [ -n "$id" ] || continue
        if ! is_installed "$id"; then
            log "跳过（未安装）: $id"
            continue
        fi
        if mount_lib "$id" 2>/dev/null; then
            ok=$((ok + 1))
        else
            fail=$((fail + 1))
            log "挂载失败 $id"
        fi
        IFS=' '
    done
    IFS="$old_ifs"
    echo "apply 完成: 成功 $ok 个, 失败 $fail 个"
    return 0
}

# ensure-core：补装缺失的核心库（对应 Rust 版 ensure_core_cmd）
cmd_ensure_core() {
    if [ ! -f "$REPOS_FILE" ]; then
        err "读取仓库列表失败: $REPOS_FILE 不存在"
        return 1
    fi
    local done=0
    # 遍历 repos.json 中 core=true 的库
    local old_ifs="$IFS"
    IFS='
'
    # 用 awk 提取 core=true 的 id 列表
    for id in $(awk '
    function trim(s){ gsub(/^[ \t\r\n]+|[ \t\r\n]+$/, "", s); return s }
    function unq(s){ gsub(/^"|"$/, "", s); return s }
    BEGIN{ m=0; c=0 }
    {
        if (match($0, /"[A-Za-z0-9_-]+"[ \t]*:/)) {
            key=unq(trim(substr($0, RSTART+1, RLENGTH-2)))
            val=trim(substr($0, RSTART+RLENGTH)); gsub(/[,\{\}]/,"",val)
            if (key=="id") { m=unq(trim(val)) }
            if (m!="" && key=="core" && val=="true") { print m; m="" }
        }
    }
    ' "$REPOS_FILE"); do
        IFS="$old_ifs"
        [ -n "$id" ] || continue
        if is_installed "$id"; then
            continue
        fi
        log "自动安装核心库: $id"
        if install_lib "$id" 0; then
            done=$((done + 1))
        else
            log "核心库安装失败: $id"
        fi
        IFS='
'
    done
    IFS="$old_ifs"
    echo "核心库就绪检查完成，新装 $done 个"
    return 0
}

# reset：卸载全部 + 清空状态（对应 Rust 版 reset_cmd）
cmd_reset() {
    unmount_all || true
    write_state '{"libs":{}}' || true
    echo "已全部卸载并清空状态"
    return 0
}

# config [mirror <url>]：读取/设置配置（对应 Rust 版 config_cmd）
cmd_config() {
    local action="$1"
    local value="$2"
    ensure_config
    case "$action" in
        "")
            # 读取并输出配置 JSON
            local mirror arch
            mirror=$(get_config "mirror" "https://packages.termux.dev/apt/termux-main")
            arch=$(get_config "arch" "aarch64")
            echo "{"
            echo "  \"mirror\": \"$mirror\","
            echo "  \"arch\": \"$arch\""
            echo "}"
            ;;
        mirror)
            if [ -z "$value" ]; then
                err "config 用法: libman config [mirror <url>]"
                return 1
            fi
            set_config "mirror" "$value" || return 1
            echo "已保存 mirror = $value"
            ;;
        *)
            err "config 用法: libman config [mirror <url>]"
            return 1
            ;;
    esac
    return 0
}

# version：打印版本
cmd_version() {
    echo "libman $VERSION"
    return 0
}

# -----------------------------------------------------------------------------
# hide：Magisk / root 检测隐藏（对应 Rust 版 hide.rs，一次性执行无驻留）
# -----------------------------------------------------------------------------
HIDE_STATE="$MODDIR/hide/state.json"
HIDE_STUB="$MODDIR/hide/stub"

# 已覆盖的路径列表（从 hide/state.json 解析出引号内的 / 开头路径）
hide_applied() {
    [ -f "$HIDE_STATE" ] || return 0
    tr -d '\n' < "$HIDE_STATE" 2>/dev/null | grep -oE '"/[^"]*"' | tr -d '"'
}

# hide apply：bind mount 空文件覆盖存在的检测路径（幂等，只补缺）
cmd_hide_apply() {
    mkdir -p "$HIDE_STUB" 2>/dev/null || true
    local already applied=0 skipped=0 p
    already=$(hide_applied)
    for p in /system/bin/su /system/xbin/su /sbin/su /debug_ramdisk/su \
             /system/bin/magisk /system/bin/magiskinit /sbin/magisk \
             /sbin/magiskinit /sbin/.magisk; do
        [ -e "$p" ] || continue                      # 系统里没有的跳过
        if echo "$already" | grep -qx "$p"; then
            continue                                  # 已由我们覆盖
        fi
        local base="${p##*/}"
        local stub="$HIDE_STUB/$base"
        : > "$stub" 2>/dev/null
        if mount --bind "$stub" "$p" 2>/dev/null; then
            echo "$p" >> "$HIDE_STATE"
            applied=$((applied + 1))
            echo "已隐藏: $p"
        else
            skipped=$((skipped + 1))
            log "hide 跳过: $p"
        fi
    done
    [ -n "$already" ] && { echo "$already" >> "$HIDE_STATE" 2>/dev/null; }
    # 去重（避免重复执行时 state 里出现重复路径）
    local uniq
    uniq=$(hide_applied | sort -u)
    : > "$HIDE_STATE"
    for p in $uniq; do echo "$p" >> "$HIDE_STATE"; done
    if [ "$applied" -eq 0 ] && [ -z "$uniq" ]; then
        err "没有可覆盖的路径（可能设备无这些检测点或不支持 bind mount）"
        return 1
    fi
    log "hide apply: 本次覆盖 $applied 个，跳过 $skipped 个"
    echo "隐藏完成：本次覆盖 $applied 个路径"
    return 0
}

# hide restore：umount 我们覆盖过的路径并清理
cmd_hide_restore() {
    local applied ok=0 p
    applied=$(hide_applied)
    for p in $applied; do
        umount "$p" 2>/dev/null && ok=$((ok + 1))
    done
    rm -rf "$HIDE_STUB" 2>/dev/null || true
    : > "$HIDE_STATE"
    log "hide restore: 还原 $ok 个路径"
    echo "已还原 $ok 个隐藏路径"
    return 0
}

# hide status：查看当前覆盖状态
cmd_hide_status() {
    local applied
    applied=$(hide_applied)
    local n=0 p
    for p in $applied; do n=$((n + 1)); done
    echo "{ \"applied\": $n }"
    for p in $applied; do echo "  $p"; done
    return 0
}

# help：打印用法
cmd_help() {
    cat <<'EOF'
用法: libman <命令> [参数]

命令:
  list                列出所有库及状态（JSON）
  status              简要状态（JSON）
  install <id>        下载并安装扩展库
  remove <id>         删除已安装的库（含卸载）
  mount <id>          挂载指定库到 /system/bin、/system/lib（即时生效，无需重启）
  unmount <id>        卸载指定库
  toggle <id>         一键开关（缺库自动安装）
  apply               按已保存状态重放挂载（开机用）
  ensure-core         补装缺失的核心库（安装/开机用）
  reset               卸载全部并清理
  hide apply          隐藏 Magisk/root 检测痕迹（bind 覆盖，一次性无驻留）
  hide restore        还原 hide 覆盖
  hide status         查看当前隐藏状态
  config <key> <val>  读取/设置配置（如 mirror）
  version             打印版本
EOF
    return 0
}

# -----------------------------------------------------------------------------
# 主入口：按 $1 路由到对应子命令
# -----------------------------------------------------------------------------
CMD="$1"
shift 2>/dev/null || true

logfile "执行命令: ${CMD:-<空>} $*"

case "$CMD" in
    list)         cmd_list ;;
    status)       cmd_status ;;
    install)      cmd_install "$1" ;;
    remove)       cmd_remove "$1" ;;
    mount)        cmd_mount "$1" ;;
    unmount)      cmd_unmount "$1" ;;
    toggle)       cmd_toggle "$1" ;;
    apply)        cmd_apply ;;
    ensure-core)  cmd_ensure_core ;;
    reset)        cmd_reset ;;
    config)       cmd_config "$1" "$2" ;;
    version)      cmd_version ;;
    help|--help|-h) cmd_help ;;
    "")
        err "缺少命令"
        cmd_help >&2
        exit 1
        ;;
    *)
        err "未知命令: $CMD"
        cmd_help >&2
        exit 1
        ;;
esac
exit 0
