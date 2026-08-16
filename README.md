<p align="center">
  <img src="https://cdn.jsdelivr.net/gh/MineACEx/libpool@master/webroot/icons/icon.png" width="112" height="112" alt="LibPool · 库池" />
</p>

<h1 align="center">LibPool · 库池</h1>

<p align="center">
  <b>补齐 Android 默认缺失的常用/热门库，一键挂载到 <code>/system/bin</code>（及 <code>/system/lib</code>）</b><br />
  内置 KsuWebUI：233 个扩展库 · 亮暗主题 · 自定义背景 · 云更新 · Rust 原生低功耗
</p>

<p align="center">[English](README_EN.md) · 简体中文</p>

<p align="center">
  <a href="https://github.com/MineACEx/libpool/blob/master/LICENSE"><img src="https://img.shields.io/badge/License-Apache--2.0-blue.svg" alt="License" /></a>
  <a href="https://github.com/MineACEx/libpool#安装"><img src="https://img.shields.io/badge/Platform-Magisk%20%7C%20KernelSU%20%7C%20APatch-orange.svg" alt="Platform" /></a>
  <a href="https://github.com/MineACEx/libpool#使用"><img src="https://img.shields.io/badge/UI-KsuWebUI-0099ff.svg" alt="KsuWebUI" /></a>
  <a href="https://github.com/MineACEx/libpool/releases"><img src="https://img.shields.io/github/v/release/MineACEx/libpool?color=0071e3&amp;label=Release" alt="Release" /></a>
  <a href="https://github.com/MineACEx/libpool/releases"><img src="https://img.shields.io/github/downloads/MineACEx/libpool/total?color=34c759&amp;label=Downloads" alt="Downloads" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/stars/MineACEx/libpool?color=e5a00d&amp;label=Stars" alt="Stars" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/forks/MineACEx/libpool?color=5856d6&amp;label=Forks" alt="Forks" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/last-commit/MineACEx/libpool?label=Last%20commit" alt="Last commit" /></a>
  <a href="https://github.com/MineACEx/libpool/issues"><img src="https://img.shields.io/github/issues/MineACEx/libpool?color=ff453a&amp;label=Issues" alt="Issues" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/languages/top/MineACEx/libpool?label=Language" alt="Language" /></a>
  <a href="https://github.com/MineACEx/libpool"><img src="https://img.shields.io/github/languages/code-size/MineACEx/libpool?label=Code%20size" alt="Code size" /></a>
</p>

支持：**Magisk** / **KernelSU** / **APatch**，兼容 **32 位（armv7）** 设备。

---

## 特性

- **233 个扩展库**（10 大分类），含常用实用库：`adb` / `fastboot`（android-tools）、`curl`、`git`、`htop`、`strace`、`gdb`、`tmux`、`rsync`、`jq`、`ffmpeg`、`nodejs`、`go`、`rust`、`python` 等
- **18 个核心自带库**（curl / git / tar / unzip / awk 等基础库），开机自动就位
- **自动跟随最新版**：库从 Termux 官方/镜像仓库解析最新版本安装，无需手动更新
- **标准 magic mount**：库文件写入模块 `system/` 目录，由 Magisk / KernelSU / APatch 开机叠加到 `/system`，新增命令开箱即用（无需写入只读的 /system）；系统已有同名命令则 bind mount 即时覆盖。开机自动重放挂载
- **挂载即时生效、无需重启**：`mount` / `unmount` 走 overlayfs 热挂载，命令执行完立刻可用的库；配合开机自动重放保证重启后配置不丢
- **系统已有库绝不覆盖**：挂载前先检查 `/system` 里是否已存在同名 bin / lib，存在则跳过，绝不覆盖系统原生文件
- **按应用深度隐藏 root**：把银行 / MOMO / 淘宝等会检测 root 的应用加进 KernelSU denylist，运行时彻底隐藏 su 痕迹，开关即生效无需重启；可从已装应用里自选，图标读取真实应用图标
- **精简自由**：不想要的库一键删除，释放空间
- **低功耗零常驻**：管理工具为 Rust 静态二进制，仅安装/挂载时运行后退出，空闲占用趋近 0
- **32 位兼容**：armv7 设备自动使用 32 位原生二进制，其它架构回退 shell 脚本
- **云更新**：对比本地与云端版本号，发现新版本时弹窗提示，下载链接一键跳转默认浏览器
- **美观的 WebUI**：iOS 18 通透毛玻璃 + G2 连续圆角，触摸优先动效（涟漪/按压/打断/滚动入场），亮暗主题，自定义壁纸模糊度，适配手机与平板
- **统一日志**：安装 / 服务 / WebUI / 工具运行日志全部整齐存放在模块目录 `log/` 文件夹，隐藏性与安全性兼顾，方便问题追溯

## 用 Rust 干了什么

`tools/libman` 是用 **Rust** 编译的原生管理工具（`aarch64-unknown-linux-musl` 与 `armv7-unknown-linux-musleabihf` 纯静态二进制，零系统依赖）。它承担模块的全部核心逻辑：

| 命令 | 职责 | 为何用 Rust |
|------|------|-------------|
| `list` / `status` | 读取 `state.json` 与已安装库，输出 WebUI 需要的 JSON | 解析快、无解释器开销 |
| `install` | 从 Termux 仓库解析最新版 → 下载 `.deb` → 解包（内置 ar/JSON/xz 解析）→ 提取 `bin`/`lib` 安装 | 纯逻辑，Rust 内存安全无崩溃风险 |
| `mount` / `unmount` | `mount --bind` 到 `/system/bin`、`/system/lib`，即时生效 | 系统调用稳定、权限可控 |
| `toggle` | 一键开关（缺库自动安装） | 原子化状态切换 |
| `apply` | 开机按已保存状态幂等重放挂载 | 本地操作，快速且不阻塞开机 |
| `ensure-core` | 补装缺失的核心库 | 后台运行，不阻塞 |
| `reset` | 卸载全部并清空状态 | 容错清理 |
| `config` | 读写镜像等配置 | 极简 JSON 序列化 |
| `hide apply/restore` | 隐藏 Magisk/root 检测痕迹（bind 覆盖常见检测路径） | 一次性命令、无驻留、可完全撤销 |
| `hide apps` | 按应用深度隐藏（扫描已装应用 / 读写列表 / 写入 KernelSU denylist / 开机重放） | 与 WebUI 打通，权限可控 |

**为什么用 Rust 而不是 shell：** shell 脚本每个操作都要 fork 解释器、字符串处理脆弱、JSON 解析易错；Rust 编译后是一个几 KB 级可执行文件，启动毫秒级、内存占用极小、永不因脚本错误半途崩溃。`tools/libman.sh` 仅作为极端环境（无二进制可执行）的兜底保留。

## 安装

1. 下载 [Releases](./releases) 里的 `libpool-<version>.zip`（大陆用户可用 Release 页里提供的国内加速镜像链接）
2. 打开 Magisk / KernelSU 管理器 → 模块 → 从本地安装 → 选择该 zip
3. 重启（或打开 KernelSU 的模块 WebUI 直接开始管理）

> 首次开机后台自动安装核心库（约 18 个），可在 WebUI「已挂载」页看到进度。

## 使用

打开 **KernelSU 管理器 → 模块 → LibPool → WebUI**：

| 页面 | 说明 |
|------|------|
| 已挂载 | 查看已下载/已挂载的库，一键开关、删除精简 |
| 扩展库 | 浏览 233 个库，搜索、按分类筛选、一键下载 |
| 深度隐藏 | 添加自己手机里已安装的应用（真实图标），把银行/检测 root 的应用加进 KernelSU denylist 彻底隐藏 root，开关即时生效 |
| 设置 | 亮/暗主题、自定义背景（网址或相册）与壁纸模糊度、下载镜像、全部重置 |

### 公告（可选）

模块目录 `/.git/url.txt` 是公告配置文件，**可直接手动编辑**：
写入一个网址（一行一个，取第一个有效 `http(s)://` 链接），WebUI 启动后每天最多弹一次公告。
内容为 **Markdown** 文本（支持标题、粗体/斜体、列表、引用、代码、链接）。留空或删除该文件则不显示公告。
默认指向本仓库的 `announce.md`，你也可以换成自己的链接。

### 云更新（可选）

- `/.git/update.txt`：云端**版本号**纯文本网址（内容为一个版本号，如 `1.1.0`）。WebUI 启动时拉取并与本地 `module.prop` 的 `version=` 对比，不一致则弹出更新提示（外观与普通公告一致）。
- `/.git/download.txt`：**下载/更新页面**网址。点击更新弹窗中的蓝色下划线链接，会通过系统默认浏览器打开。
- 以上文件都留空或删除时，回退使用 WebUI 内置常量（`app.js` 中的 `UPDATE_URL` / `UPDATE_DOWNLOAD_URL` / `DEFAULT_RELEASES_URL`）。
- 同一云端版本只提醒一次，出现更新的云端版本时会再次提醒。

### 下载镜像

大陆网络下，建议在「设置 → 下载镜像」选择清华或北外镜像，速度更稳。

## 架构兼容

| 设备架构 | 使用方式 |
|----------|----------|
| aarch64 / arm64 | `tools/libman`（原生） |
| armv7 / arm（32 位） | `tools/libman-arm`（32 位原生） |
| 其它（x86 等） | `tools/libman.sh`（shell 兜底） |

脚本会在安装/开机时自动按 `uname -m` 选择，无需手动配置。

## 构建

### 前置

- Rust 工具链（stable）
- 交叉编译目标：
  ```bash
  rustup target add aarch64-unknown-linux-musl        # 64 位
  rustup target add armv7-unknown-linux-musleabihf    # 32 位（可选）
  ```

### Windows

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
```

### Linux / macOS

```bash
sh scripts/build.sh
```

产物：`dist/libpool-<version>.zip`。

## 目录结构

```
libpool/
├── module.prop            # 模块元数据
├── customize.sh           # 安装：初始化 + 架构选择 + 后台装核心库
├── service.sh             # 开机：重放挂载（timeout 保护）+ 后台补装核心库
├── post-fs-data.sh        # 早期启动占位（极轻量，不阻塞）
├── uninstall.sh           # 卸载：释放全部 bind mount
├── sepolicy.rule          # SELinux（最小权限）
├── module-config/         # 源码层公告/云更新配置（url.txt / update.txt / download.txt）
│                          # 打包进模块后置于模块目录的 .git/ 下，用户可手动编辑
├── system/bin|lib         # magic mount 目标
├── libs/<id>/bin|lib      # 实际库文件（安装后生成）
├── hide/apps.json         # 深度隐藏应用列表（自选已装应用，真实图标）
├── log/                   # 统一日志目录（libman / service / webui 日志）
├── tools/
│   ├── libman             # Rust 原生管理工具（aarch64）
│   ├── libman-arm         # Rust 原生管理工具（armv7 32 位）
│   └── libman.sh          # shell 兜底（极端环境）
├── webroot/               # KsuWebUI（完全离线可用）
│   └── data/repos.json    # 233 库数据库
├── repo_src/              # 构建源数据
├── src/rust-libman/       # Rust 源码
└── scripts/               # 构建脚本
```

## 常见问题

**Q：模块会不会拖慢开机 / 卡开机？**
不会。`customize.sh` 数秒内返回，`service.sh` 的挂载重放有 `timeout 60` 保护、核心库补装完全后台。任何环节失败只写日志，**禁用本模块即可完全恢复开机**。

**Q：模块耗电 / 占资源吗？**
不。模块没有守护进程、没有轮询、没有开机自启常驻，空闲时 CPU/内存占用趋近 0。

**Q：32 位手机能用吗？**
能。armv7 设备自动用 `libman-arm`，其它架构回退 shell 兜底。

**Q：为什么有些库装了但命令找不到？**
某些库（如 python）会同时安装依赖的动态库到 `/system/lib`。若仍提示找不到，可在 WebUI 中重新开关一次该库。

**Q：大陆下载很慢怎么办？**
在「设置」里切换到清华或北外镜像。

**Q：担心模块精简后系统出问题？**
模块只往 `/system/bin` 挂载你手动开启的库，不修改系统分区。删除库也不会影响系统原有文件。

**Q：想反馈 Bug 或提需求？**
请在 [Issues](./issues) 新建 Issue，尽量附上：机型/安卓版本/架构（`uname -m`）、模块版本号、复现步骤、以及 WebUI 控制台日志。

## 贡献

欢迎提交 Pull Request 与 Issue。代码风格遵循 Apache-2.0 许可，所有源码文件均声明作者（MINO · Himer）。

## 许可证

本项目使用 **Apache License 2.0** 开源。详见 [LICENSE](./LICENSE)。
