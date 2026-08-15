# -*- coding: utf-8 -*-
# LibPool · 库池 — 扩充脚本
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
"""一次性脚本：将 repos.json 从 136 扩充到 280+，校验后写回。"""
import json, io, sys, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PATH = os.path.join(ROOT, "repo_src", "repos.json")

with io.open(PATH, "r", encoding="utf-8") as f:
    existing = json.load(f)

existing_ids = {e["id"] for e in existing}
print(f"现有: {len(existing)} 个库")

# 新增库（均为 type=termux，可在线下载；core 默认 false）
NEW = [
    # ---------- 编程语言 ----------
    ("bun", "Bun", "极快的现代 JavaScript/TypeScript 运行时与打包工具", "编程语言", "bun"),
    ("swift", "Swift", "苹果出品的现代编程语言编译器", "编程语言", "swift"),
    ("rlang", "R 语言", "统计分析、数据科学专用的开源编程语言", "编程语言", "r"),
    ("tcl", "Tcl/Tk", "轻量脚本语言，常用于工具与 GUI 脚本", "编程语言", "tcl"),
    ("guile", "GNU Guile", "GNU 官方的 Scheme 方言解释器", "编程语言", "guile"),
    ("fpc", "Free Pascal", "免费的 Pascal 编译器与开发环境", "编程语言", "fpc"),
    ("openjdk", "OpenJDK", "Java 开发与运行所需的完整 JDK", "编程语言", "openjdk-17"),
    ("vlang", "V 语言", "简洁快速的系统编程语言编译器", "编程语言", "vlang"),
    ("gnucobol", "GnuCOBOL", "古老而稳重的 COBOL 语言编译器", "编程语言", "gnucobol"),
    ("fortran", "Fortran", "科学计算领域经典的编译型语言", "编程语言", "gcc-fortran"),
    ("ghc", "GHC", "Haskell 语言的官方编译器", "编程语言", "ghc"),
    ("vala", "Vala", "面向 GNOME 的现代面向对象语言", "编程语言", "vala"),
    ("octave", "GNU Octave", "开源数值计算与 MATLAB 兼容环境", "编程语言", "octave"),
    ("swi-prolog", "SWI-Prolog", "人工智能领域常用的逻辑编程语言", "编程语言", "swi-prolog"),
    ("sbcl", "SBCL", "高性能的 Common Lisp 编译器", "编程语言", "sbcl"),
    ("powershell", "PowerShell", "微软出品的跨平台自动化命令行", "编程语言", "powershell"),
    # ---------- 网络工具 ----------
    ("aria2", "aria2", "多线程断点续传的下载加速工具", "网络工具", "aria2"),
    ("axel", "Axel", "轻量级多线程命令行下载加速器", "网络工具", "axel"),
    ("lftp", "LFTP", "功能强大的命令行 FTP/HTTP 客户端", "网络工具", "lftp"),
    ("tor", "Tor", "匿名网络接入客户端（洋葱路由）", "网络工具", "tor"),
    ("proxychains-ng", "Proxychains-ng", "让任意程序走代理的隧道工具", "网络工具", "proxychains-ng"),
    ("mosquitto", "Mosquitto", "轻量可靠的 MQTT 消息代理服务器", "网络工具", "mosquitto"),
    ("masscan", "Masscan", "号称全网最快的端口扫描工具", "网络工具", "masscan"),
    ("whois", "Whois", "查询域名与 IP 归属信息的工具", "网络工具", "whois"),
    ("dnsutils", "DNS Utils", "提供 dig/nslookup 等 DNS 查询工具", "网络工具", "dnsutils"),
    ("wireguard-tools", "WireGuard 工具", "新一代极简 VPN 的配置与管理工具", "网络工具", "wireguard-tools"),
    ("openvpn", "OpenVPN", "成熟稳定的开源 VPN 客户端", "网络工具", "openvpn"),
    ("sshpass", "sshpass", "用密码自动化 SSH 登录的小工具", "网络工具", "sshpass"),
    ("caddy", "Caddy", "自动 HTTPS 的现代轻量 Web 服务器", "网络工具", "caddy"),
    ("iftop", "iftop", "实时查看各连接的带宽占用", "网络工具", "iftop"),
    ("nload", "nload", "直观显示网卡实时流量图表", "网络工具", "nload"),
    ("net-tools", "Net-tools", "ifconfig/netstat 等经典网络工具", "网络工具", "net-tools"),
    ("inetutils", "Inetutils", "telnet/ftp/ping 等基础网络套件", "网络工具", "inetutils"),
    ("telnet", "Telnet", "经典远程终端连接协议客户端", "网络工具", "telnet"),
    ("vsftpd", "vsftpd", "轻量安全的 FTP 文件服务器", "网络工具", "vsftpd"),
    ("samba", "Samba", "Windows 文件共享（SMB）服务器", "网络工具", "samba"),
    ("frp", "frp", "内网穿透神器，把内网服务映射到公网", "网络工具", "frp"),
    ("lrzsz", "lrzsz", "rz/sz 文件传输工具（配合终端串口）", "网络工具", "lrzsz"),
    # ---------- 压缩打包 ----------
    ("rar", "RAR", "官方 RAR 压缩与解压工具", "压缩打包", "rar"),
    ("lha", "LHA", "经典 .lzh 压缩格式工具", "压缩打包", "lha"),
    ("cabextract", "cabextract", "解压微软 CAB 安装包工具", "压缩打包", "cabextract"),
    ("cpio", "cpio", "Unix 经典归档交换格式工具", "压缩打包", "cpio"),
    ("lzop", "LZOP", "速度极快的 lzo 压缩工具", "压缩打包", "lzop"),
    ("xdelta3", "xdelta3", "二进制增量补丁生成与应用工具", "压缩打包", "xdelta3"),
    ("pigz", "Pigz", "并行版 gzip，多核压缩更快", "压缩打包", "pigz"),
    ("pbzip2", "pbzip2", "并行版 bzip2 压缩工具", "压缩打包", "pbzip2"),
    ("squashfs-tools", "SquashFS 工具", "只读压缩文件系统制作工具", "压缩打包", "squashfs-tools"),
    ("erofs-utils", "EROFS 工具", "安卓系统使用的只读文件系统工具", "压缩打包", "erofs-utils"),
    ("zopfli", "Zopfli", "压缩率更高的 DEFLATE 算法工具", "压缩打包", "zopfli"),
    ("bsdtar", "libarchive", "支持超多格式的现代 tar/归档工具", "压缩打包", "libarchive"),
    # ---------- 文本处理 ----------
    ("neovim", "Neovim", "新一代极简强大的 Vim 分支编辑器", "文本处理", "neovim"),
    ("micro", "Micro", "现代易用的终端编辑器，开箱即用", "文本处理", "micro"),
    ("joe", "Joe", "传统易上手的终端文本编辑器", "文本处理", "joe"),
    ("ripgrep", "ripgrep", "极速递归搜索文本（rg）", "文本处理", "ripgrep"),
    ("fd", "fd", "更快更好用的 find 替代工具", "文本处理", "fd"),
    ("fzf", "fzf", "命令行模糊搜索神器", "文本处理", "fzf"),
    ("bat", "bat", "带语法高亮的 cat 增强工具", "文本处理", "bat"),
    ("eza", "eza", "带图标和颜色的现代 ls 替代", "文本处理", "eza"),
    ("tldr", "tldr", "简明版的命令帮助文档", "文本处理", "tldr"),
    ("glow", "Glow", "终端里渲染 Markdown 文档", "文本处理", "glow"),
    ("pandoc", "Pandoc", "万能的文档格式转换器", "文本处理", "pandoc"),
    ("universal-ctags", "Universal Ctags", "代码标签索引生成工具", "文本处理", "universal-ctags"),
    ("hexedit", "Hexedit", "终端十六进制文件编辑器", "文本处理", "hexedit"),
    ("translate-shell", "translate-shell", "终端里直接翻译的 shell 工具", "文本处理", "translate-shell"),
    ("xsv", "xsv", "超快的 CSV 数据处理工具", "文本处理", "xsv"),
    ("man-db", "man-db", "经典的手册页（man）查看系统", "文本处理", "man-db"),
    ("the_silver_searcher", "The Silver Searcher", "代码搜索神器 ag", "文本处理", "the_silver_searcher"),
    ("recode", "Recode", "字符集与换行格式转换工具", "文本处理", "recode"),
    # ---------- 系统工具 ----------
    ("busybox", "BusyBox", "一站式常用 Linux 命令合集", "系统工具", "busybox"),
    ("clang", "Clang", "LLVM 高性能 C/C++ 编译器", "系统工具", "clang"),
    ("ninja", "Ninja", "极快的轻量构建系统", "系统工具", "ninja"),
    ("meson", "Meson", "快速友好的现代构建系统", "系统工具", "meson"),
    ("pkgconf", "pkg-config", "库编译参数自动查找工具", "系统工具", "pkgconf"),
    ("autoconf", "Autoconf", "自动化生成 configure 脚本", "系统工具", "autoconf"),
    ("automake", "Automake", "自动化生成 Makefile 的工具", "系统工具", "automake"),
    ("libtool", "Libtool", "便携式共享库构建工具", "系统工具", "libtool"),
    ("bison", "Bison", "yacc 兼容的语法分析器生成器", "系统工具", "bison"),
    ("flex", "Flex", "词法分析器快速生成工具", "系统工具", "flex"),
    ("file", "File", "识别文件真实类型", "系统工具", "file"),
    ("ncdu", "ncdu", "终端下的磁盘占用分析器", "系统工具", "ncdu"),
    ("btop", "btop", "图形化的系统资源监控面板", "系统工具", "btop"),
    ("bottom", "bottom", "轻量美观的系统监控工具", "系统工具", "bottom"),
    ("duf", "duf", "一目了然的磁盘使用情况", "系统工具", "duf"),
    ("dust", "dust", "更直观的磁盘占用统计", "系统工具", "dust"),
    ("iotop", "iotop", "实时查看磁盘读写进程", "系统工具", "iotop"),
    ("nethogs", "NetHogs", "按进程查看网络流量", "系统工具", "nethogs"),
    ("inotify-tools", "Inotify 工具", "监听文件变化并触发命令", "系统工具", "inotify-tools"),
    ("apktool", "Apktool", "反编译与重打包 APK 的工具", "系统工具", "apktool"),
    ("jadx", "jadx", "APK 反编译为 Java 源码", "系统工具", "jadx"),
    ("termux-api", "Termux API", "调用手机传感器等系统接口", "系统工具", "termux-api"),
    # ---------- 数据库 ----------
    ("sqlcipher", "SQLCipher", "带加密功能的 SQLite 扩展", "数据库", "sqlcipher"),
    ("couchdb", "CouchDB", "面向文档的 NoSQL 数据库", "数据库", "couchdb"),
    ("etcd", "etcd", "分布式系统用的键值数据库", "数据库", "etcd"),
    ("rqlite", "rqlite", "轻量分布式 SQLite 集群", "数据库", "rqlite"),
    # ---------- 多媒体 ----------
    ("x264", "x264", "最流行的 H.264 视频编码器", "多媒体", "x264"),
    ("x265", "x265", "高效率 H.265/HEVC 视频编码器", "多媒体", "x265"),
    ("lame", "LAME", "高质量的 MP3 音频编码器", "多媒体", "lame"),
    ("flac", "FLAC", "无损音频编码与解码工具", "多媒体", "flac"),
    ("opus-tools", "Opus 工具", "现代高效音频编码工具", "多媒体", "opus-tools"),
    ("vorbis-tools", "Vorbis 工具", "开源 Ogg Vorbis 音频工具", "多媒体", "vorbis-tools"),
    ("mkvtoolnix", "MKVToolNix", "MKV 视频封装/拆解与字幕处理", "多媒体", "mkvtoolnix"),
    ("atomicparsley", "AtomicParsley", "MP4/M4A 元数据标签编辑工具", "多媒体", "atomicparsley"),
    ("youtube-dl", "youtube-dl", "经典的开源视频下载工具", "多媒体", "youtube-dl"),
    ("streamlink", "Streamlink", "把直播流保存为视频文件", "多媒体", "streamlink"),
    ("webp", "WebP 工具", "谷歌 WebP 图片格式转换工具", "多媒体", "webp"),
    ("pngquant", "pngquant", "有损压缩 PNG 减小体积", "多媒体", "pngquant"),
    ("dcraw", "dcraw", "读取 RAW 相机照片的工具", "多媒体", "dcraw"),
    ("ffmpegthumbnailer", "FFmpeg Thumbnailer", "视频一键生成缩略图", "多媒体", "ffmpegthumbnailer"),
    ("mpd", "MPD", "轻量的音乐播放守护进程", "多媒体", "mpd"),
    ("cmus", "cmus", "功能强大的终端音乐播放器", "多媒体", "cmus"),
    # ---------- 安全工具 ----------
    ("nuclei", "Nuclei", "基于模板的快速漏洞扫描器", "安全工具", "nuclei"),
    ("subfinder", "Subfinder", "快速收集子域名的工具", "安全工具", "subfinder"),
    ("amass", "Amass", "深度资产与子域枚举工具", "安全工具", "amass"),
    ("ffuf", "ffuf", "极快的 Web 目录/参数模糊测试器", "安全工具", "ffuf"),
    ("responder", "Responder", "局域网 LLMNR/NBT-NS 毒化工具", "安全工具", "responder"),
    ("sherlock", "Sherlock", "跨平台社交账号用户名搜索", "安全工具", "sherlock"),
    ("theharvester", "theHarvester", "被动信息收集与邮箱搜集", "安全工具", "theharvester"),
    ("wafw00f", "wafw00f", "识别网站是否套了 WAF", "安全工具", "wafw00f"),
    ("sublist3r", "Sublist3r", "子域名快速枚举工具", "安全工具", "sublist3r"),
    ("exploitdb", "ExploitDB", "searchsploit 漏洞利用数据库检索", "安全工具", "exploitdb"),
    ("yara", "YARA", "恶意软件特征模式匹配引擎", "安全工具", "yara"),
    ("clamav", "ClamAV", "开源杀毒软件扫描引擎", "安全工具", "clamav"),
    ("lynis", "Lynis", "系统安全审计与加固建议", "安全工具", "lynis"),
    ("macchanger", "macchanger", "修改网卡 MAC 地址", "安全工具", "macchanger"),
    ("bettercap", "bettercap", "模块化的中间人攻击框架", "安全工具", "bettercap"),
    ("dsniff", "dsniff", "arpspoof 等网络嗅探套件", "安全工具", "dsniff"),
    ("fcrackzip", "fcrackzip", "ZIP 压缩包密码爆破", "安全工具", "fcrackzip"),
    ("gnupg", "GnuPG", "文件加密与签名（gpg）", "安全工具", "gnupg"),
    ("age", "age", "简单现代的加密工具", "安全工具", "age"),
    # ---------- 文件系统 ----------
    ("xfsprogs", "XFS 工具", "XFS 文件系统管理工具", "文件系统", "xfsprogs"),
    ("btrfs-progs", "Btrfs 工具", "Btrfs 文件系统管理工具", "文件系统", "btrfs-progs"),
    ("f2fs-tools", "F2FS 工具", "安卓闪存友好的文件系统工具", "文件系统", "f2fs-tools"),
    ("dosfstools", "dosfstools", "FAT/exFAT 分区格式化工具", "文件系统", "dosfstools"),
    ("e2tools", "e2tools", "不挂载直接操作 ext2/3/4 分区", "文件系统", "e2tools"),
    ("cryptsetup", "cryptsetup", "LUKS 磁盘加密工具", "文件系统", "cryptsetup"),
    # ---------- 其他工具 ----------
    ("fastfetch", "fastfetch", "超炫的系统信息展示工具", "其他工具", "fastfetch"),
    ("cpufetch", "cpufetch", "在终端画出 CPU 芯片信息", "其他工具", "cpufetch"),
    ("toilet", "toilet", "彩色的大字艺术字工具", "其他工具", "toilet"),
    ("taskwarrior", "Taskwarrior", "命令行待办事项管理", "其他工具", "taskwarrior"),
    ("calcurse", "calcurse", "终端里的日程与待办日历", "其他工具", "calcurse"),
    ("newsboat", "Newsboat", "终端 RSS/Atom 阅读器", "其他工具", "newsboat"),
    ("w3m", "w3m", "终端里的网页浏览器", "其他工具", "w3m"),
    ("lynx", "Lynx", "经典的纯文本网页浏览器", "其他工具", "lynx"),
    ("vifm", "Vifm", "双栏终端文件管理器", "其他工具", "vifm"),
    ("ranger", "Ranger", "带预览的终端文件管理器", "其他工具", "ranger"),
    ("mc", "Midnight Commander", "老牌双栏文件管理器", "其他工具", "mc"),
    ("shellcheck", "ShellCheck", "Shell 脚本静态检查", "其他工具", "shellcheck"),
    ("shfmt", "shfmt", "Shell 脚本格式化工具", "其他工具", "shfmt"),
    ("delta", "delta", "Git diff 高亮增强工具", "其他工具", "delta"),
    ("lazygit", "LazyGit", "终端里的 Git 图形操作面板", "其他工具", "lazygit"),
    ("tig", "tig", "终端里的 Git 浏览工具", "其他工具", "tig"),
    ("gh", "GitHub CLI", "官方 GitHub 命令行工具", "其他工具", "gh"),
    ("glab", "GitLab CLI", "官方 GitLab 命令行工具", "其他工具", "glab"),
    ("zoxide", "zoxide", "智能记住目录的 cd 增强", "其他工具", "zoxide"),
    ("direnv", "direnv", "按目录自动加载环境变量", "其他工具", "direnv"),
    ("broot", "broot", "可视化目录树浏览工具", "其他工具", "broot"),
    ("fish", "Fish", "开箱即用友好的交互式 Shell", "其他工具", "fish"),
    ("zsh", "Zsh", "功能强大的交互式 Shell", "其他工具", "zsh"),
    ("nushell", "Nushell", "数据驱动的现代 Shell", "其他工具", "nushell"),
    ("starship", "Starship", "极简快速的跨 Shell 提示符", "其他工具", "starship"),
    ("speedtest-cli", "Speedtest CLI", "命令行网速测试工具", "其他工具", "speedtest-cli"),
    ("ansiweather", "AnsiWeather", "终端里显示天气预报", "其他工具", "ansiweather"),
    ("proot", "proot", "免 Root 模拟运行 Linux 发行版", "其他工具", "proot"),
    ("tty-clock", "tty-clock", "终端里的时钟屏保", "其他工具", "tty-clock"),
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

# 按分类分组排序，保证同分类相邻
by_cat = {}
for e in existing:
    by_cat.setdefault(e["category"], []).append(e)
order = ["编程语言", "网络工具", "压缩打包", "文本处理", "系统工具",
         "数据库", "多媒体", "安全工具", "文件系统", "其他工具"]
for cat in order:
    if cat in by_cat:
        by_cat[cat].sort(key=lambda x: x["name"].lower())
out = []
for cat in order:
    if cat in by_cat:
        out.extend(by_cat[cat])
# 未在 order 中的分类追加到末尾
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
