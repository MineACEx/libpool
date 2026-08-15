# -*- coding: utf-8 -*-
# LibPool · 库池 — 实用库扩充脚本
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
"""一次性脚本：给 repos.json 追加实用库批（adb/fastboot/运维/调试/工具链等），自动去重。"""
import json, io, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PATH = os.path.join(ROOT, "repo_src", "repos.json")

with io.open(PATH, "r", encoding="utf-8") as f:
    existing = json.load(f)
existing_ids = {e["id"] for e in existing}
print(f"现有: {len(existing)} 个库")

NEW = [
    # ---------- Android 工具（核心新增） ----------
    ("android-tools", "ADB / Fastboot", "安卓刷机调试全家桶：adb、fastboot、logcat 等（需连接设备）", "系统工具", "android-tools"),
    ("adb", "ADB", "安卓调试桥：连接手机/模拟器执行命令、装应用、抓日志", "系统工具", "android-tools"),
    ("fastboot", "Fastboot", "刷机模式工具：解锁引导、刷入 boot/recovery/镜像", "系统工具", "android-tools"),
    # ---------- 系统调试 ----------
    ("strace", "strace", "跟踪进程的系统调用，排查程序崩溃与卡顿", "系统工具", "strace"),
    ("ltrace", "ltrace", "跟踪进程的库函数调用", "系统工具", "ltrace"),
    ("gdb", "GDB", "GNU 调试器：断点、单步、查看内存与变量", "系统工具", "gdb"),
    ("htop", "htop", "交互式进程监控，比 top 更好用的系统监视器", "系统工具", "htop"),
    ("rsync", "rsync", "增量同步与备份神器，局域网/本地文件同步", "系统工具", "rsync"),
    ("tmux", "tmux", "终端复用器：一个窗口开多个会话、断线不丢任务", "系统工具", "tmux"),
    ("screen", "Screen", "老牌终端复用工具：后台挂任务、会话管理", "系统工具", "screen"),
    ("socat", "Socat", "万能数据通道：端口转发、TCP/UDP 桥接、串口调试", "系统工具", "socat"),
    ("tree", "Tree", "以树形图直观显示目录结构", "系统工具", "tree"),
    ("vim", "Vim", "经典强大的文本编辑器（vi 增强版）", "系统工具", "vim"),
    ("emacs", "Emacs", "可扩展的经典编辑器与运行环境", "系统工具", "emacs"),
    ("hyperfine", "hyperfine", "命令行基准测试工具：比较两条命令谁更快", "系统工具", "hyperfine"),
    ("iproute2", "iproute2", "现代网络配置工具：ip 命令（地址/路由/隧道）", "网络工具", "iproute2"),
    ("pciutils", "PCI 工具", "查看 PCI 设备列表（lspci）", "系统工具", "pciutils"),
    ("usbutils", "USB 工具", "查看 USB 设备列表（lsusb）", "系统工具", "usbutils"),
    ("sysstat", "Sysstat", "系统性能统计：iostat、sar、mpstat、pidstat", "系统工具", "sysstat"),
    ("smartmontools", "Smartmontools", "硬盘健康监测：smartctl 查看 SMART 状态", "系统工具", "smartmontools"),
    ("hdparm", "hdparm", "磁盘参数查看与设置、缓存读写测试", "系统工具", "hdparm"),
    ("fdupes", "fdupes", "查找并清理重复文件，释放空间", "文件系统", "fdupes"),
    ("pv", "PV", "管道进度条：实时显示命令处理进度", "其他工具", "pv"),
    ("cpulimit", "cpulimit", "限制进程 CPU 占用率，防止过热", "系统工具", "cpulimit"),
    ("bc", "bc", "高精度命令行计算器", "其他工具", "bc"),
    ("expect", "Expect", "自动化交互式命令行：自动应答登录密码", "其他工具", "expect"),
    ("which", "which", "查找命令所在的绝对路径", "系统工具", "which"),
    ("diffutils", "Diffutils", "经典文件对比工具：diff、cmp、diff3", "文本处理", "diffutils"),
    ("patch", "Patch", "应用与生成补丁文件（patch 命令）", "文本处理", "patch"),
    # ---------- 网络调试 ----------
    ("netcat", "netcat", "网络瑞士军刀：端口测试、TCP/UDP 传文件", "网络工具", "netcat-openbsd"),
    ("tcpdump", "tcpdump", "抓包神器：实时捕获分析网络数据包", "网络工具", "tcpdump"),
    ("nmap", "Nmap", "端口扫描与网络探测工具", "网络工具", "nmap"),
    ("mtr", "MTR", "traceroute + ping 二合一：可视化网络链路质量", "网络工具", "mtr"),
    ("iperf3", "iperf3", "网络带宽测试工具：测速局域网/宽带", "网络工具", "iperf3"),
    ("hping3", "hping3", "高级网络测试：自定义 TCP/UDP/ICMP 包", "网络工具", "hping3"),
    ("jq", "jq", "命令行 JSON 解析神器：过滤/格式化 JSON", "文本处理", "jq"),
    ("yq", "yq", "YAML/JSON/XML 命令行处理工具", "文本处理", "yq"),
    ("wget", "wget", "经典命令行下载工具，支持递归与断点", "网络工具", "wget"),
    # ---------- 编程语言 & 工具链 ----------
    ("zig", "Zig", "现代系统编程语言与交叉编译工具链", "编程语言", "zig"),
    ("nim", "Nim", "优雅高效的编译型编程语言", "编程语言", "nim"),
    ("crystal", "Crystal", "语法似 Ruby、性能似 C 的编译型语言", "编程语言", "crystal"),
    ("elixir", "Elixir", "构建在 Erlang 之上的现代函数式语言", "编程语言", "elixir"),
    ("erlang", "Erlang", "高并发容错的语言与虚拟机（OTP）", "编程语言", "erlang"),
    ("clojure", "Clojure", "运行在 JVM 上的 Lisp 方言", "编程语言", "clojure"),
    ("scala", "Scala", "融合面向对象与函数式的 JVM 语言", "编程语言", "scala"),
    ("dlang", "D 语言", "兼顾性能与生产力的系统语言", "编程语言", "dmd"),
    ("kotlin", "Kotlin", "现代 JVM 语言，安卓官方推荐", "编程语言", "kotlin"),
    ("ruby", "Ruby", "优雅的脚本语言（配合 gem）", "编程语言", "ruby"),
    ("perl", "Perl", "文本处理强悍的经典脚本语言", "编程语言", "perl"),
    ("lua", "Lua", "轻量嵌入式脚本语言", "编程语言", "lua"),
    ("go", "Go", "谷歌出品的高性能系统编程语言", "编程语言", "golang"),
    ("rust", "Rust", "内存安全的高性能系统编程语言", "编程语言", "rust"),
    ("nodejs", "Node.js", "JavaScript 运行时（npm 生态）", "编程语言", "nodejs"),
    ("make", "Make", "经典构建工具（GNU make）", "系统工具", "make"),
    ("gcc", "GCC", "GNU C/C++ 编译器", "系统工具", "gcc"),
    # ---------- 多媒体 / 文档 ----------
    ("ffmpeg", "FFmpeg", "音视频处理瑞士军刀：转换、裁剪、推流", "多媒体", "ffmpeg"),
    ("imagemagick", "ImageMagick", "命令行图像处理：转换/压缩/合成图片", "多媒体", "imagemagick"),
    ("poppler", "Poppler", "PDF 处理：pdftotext、pdfimages、pdftoppm", "文本处理", "poppler"),
    ("ghostscript", "Ghostscript", "PDF/PostScript 解析与转换引擎", "文本处理", "ghostscript"),
    ("qpdf", "qpdf", "PDF 查看与修复、加密解密、结构分析", "文本处理", "qpdf"),
    ("tesseract", "Tesseract", "开源 OCR 文字识别引擎", "其他工具", "tesseract"),
    # ---------- 数据 ----------
    ("sqlite", "SQLite", "轻量数据库命令行工具（sqlite3）", "数据库", "sqlite"),
    ("redis", "Redis", "高性能内存键值数据库", "数据库", "redis"),
    # ---------- 压缩 ----------
    ("p7zip", "7-Zip", "新一代 7z 压缩格式工具", "压缩打包", "p7zip"),
    ("zip", "Zip", "经典 ZIP 压缩与解压工具", "压缩打包", "zip"),
]

added = 0
for (eid, name, desc, cat, pkg) in NEW:
    if eid in existing_ids:
        print(f"  跳过重复: {eid}")
        continue
    existing.append({
        "id": eid, "name": name, "desc": desc, "category": cat,
        "type": "termux", "pkg": pkg, "core": False,
    })
    existing_ids.add(eid)
    added += 1

# 按分类分组排序，保持同分类相邻
by_cat = {}
for e in existing:
    by_cat.setdefault(e["category"], []).append(e)
order = ["编程语言", "网络工具", "压缩打包", "文本处理", "系统工具",
         "数据库", "多媒体", "安全工具", "文件系统", "其他工具"]
out = []
for cat in order:
    if cat in by_cat:
        by_cat[cat].sort(key=lambda x: x["name"].lower())
        out.extend(by_cat[cat])
for cat in by_cat:
    if cat not in order:
        out.extend(by_cat[cat])

with io.open(PATH, "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, indent=2)
    f.write("\n")

ids = [e["id"] for e in out]
assert len(ids) == len(set(ids)), "存在重复 id！"
print(f"新增: {added} | 总计: {len(out)} 个库")
print(f"核心自带库: {sum(1 for e in out if e['core'])}")
from collections import Counter
print("分类统计:", dict(Counter(e['category'] for e in out)))
