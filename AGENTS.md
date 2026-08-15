# AGENTS.md — LibPool（库池）Magisk/KernelSU 模块

> 本文件是「上下文压缩后仍强制执行的持久依据」。不要依赖聊天记录，关键需求与架构决策都在这里。

## 一、项目一句话目标

补齐 Android 默认缺失的常用/热门库，把它们挂载到 `/system/bin`（及 `/system/lib`），并通过一个好看的 **KsuWebUI** 让用户自由增删库：可在线下载新库（如新版 curl）、可删除模块自带库（如 Python3）做精简、提供 **322+ 扩展库** 供一键下载挂载（含 18 个随模块自带的核心库）。出品方 **MINO · Himer (MineACE)**，**Apache-2.0** 开源。

## 二、用户硬性要求（最高优先级，永远遵守）

1. **必须先用完 4 个 skill 再动手**（apple-ui-design / beautiful-ui / design-taste-frontend / impeccable），压缩上下文后也必须先读 AGENTS.md 内的技能注册表再干。
2. **KsuWebUI 要求**：
   - 亮/暗色切换；UI 一致性；适配手机与平板比例；如同 iPhone 一样简洁；
   - **通透毛玻璃**（类似 iOS 18 液体玻璃观感：半透明 + backdrop blur + 细高光边框）；
   - 中文、新手也能看懂的文案（不用晦涩专业术语）；
   - 重要按钮文字带轻微发光；菜单卡片为「浅色白模糊磨砂玻璃」风格（替换蓝透 UI）。
3. **功耗优先**：除必须用 shell 的脚本外，其余逻辑用 **Rust/C++ 编译成可执行二进制**（本项目用 Rust → 静态 aarch64 musl 二进制，内存与 CPU 开销远小于解释型脚本）。
4. 创建 **AGENTS.md + TODO.md** 用于记住用户的话。
5. 遇到方向性问题先问用户，不要瞎猜。
6. **目标平台 = 手机/平板，交互是点击（touch/pointer），不是鼠标**。动效必须触摸优先：以「按压反馈 + 点击涟漪 + 打断动画」为核心；hover 只做桌面增强（包在 `@media (hover: hover)` 内）。触摸目标 ≥44px。
7. **G2 连续圆角（iOS18 开放风）**：大圆角矩形卡片统一 `border-radius: 24px + corner-shape: squircle`（公告卡 30px、图标 16-22px；Chrome 139+ / Android WebView 支持，渐进增强，老设备自动退回普通圆角）。胶囊/开关/圆形（radius 999px）保持默认 `round` 不用 squircle。
8. **自定义背景图**：设置页可填 http(s) 图片 URL，存 localStorage `libpool-bg`，渲染到 `.custom-bg` 层（卡片之下、界面之上），毛玻璃遮罩仍在卡片之上。
9. **设备上下边框与 WebUI 平滑模糊过渡**：`body.scrolled`（scrollY>4）时 `.top-blur/.bottom-blur` 渐变增强。
10. **公告功能**：公告弹窗为毛玻璃卡片，弹出时背景 `backdrop-filter 0→22px` 丝滑渐变（仿 iOS 应用开合模糊）。内容优先从**模块目录 `/.git/url.txt`** 读取（用户可手动编辑：每行一个，取第一个有效 http(s) 链接；纯文本站点、无标签），否则回退 `ANNOUNCEMENT_URL` 常量；每天最多弹一次。
11. **品牌与许可**：出品方 MINO · Himer（MineACE）；**所有源码文件必须声明 Copyright + Apache-2.0**；模块图标 `webroot/icons/icon.png`（用户提供源 PNG）。
12. **32 位（armv7）兼容**：`tools/libman`（aarch64）+ `tools/libman-arm`（armv7）+ `tools/libman.sh`（shell 兜底）三件套；customize/service/uninstall 均按 `uname -m` 自动选择。64 位设备绝不能用 32 位二进制，反之亦然。
13. **安全性铁律（卡开机=禁用模块即可恢复）**：
    - `customize.sh` 数秒内返回，耗时任务一律 nohup 后台；
    - `service.sh` 的 apply 加 `timeout 60` 保护，ensure-core 后台 + `timeout 600`；
    - `post-fs-data.sh` 只 mkdir，绝不 mount/联网/重逻辑；
    - 无守护进程、无轮询、无自启常驻，空闲占用趋近 0；禁用/卸载即完全恢复。
14. **scrollHeight 陷阱**：html/body **绝不能设 `height:100%`**（会锁定文档滚动高度导致页面无法滚动）；用 `min-height:100dvh` 撑满视口。

## 三、已读技能注册表（必须应用，不得以上下文压缩为由跳过）

| 技能 | 本项目应用点 |
|------|-------------|
| apple-ui-design | SF Pro 字体栈 + 层级(hero/title/body/caption)；浅深配色变量(#f5f5f7 / #1d1d1f，accent #0071e3 亮 / #0a84ff 暗)；4/8/16/24 间距节奏；毛玻璃卡片(backdrop-blur 20 / radius 18-20 / 半透明 / 1px 高光边框)；胶囊按钮(radius 980px + hover 微缩放/微发光)；cubic-bezier(0.25,0.1,0.25,1) 动效；触控≥44px / 对比≥4.5:1 / 版心 ~680px / 深色模式 |
| beautiful-ui | 先理解需求再设计；5 维度(字体/颜色/布局/动画/细节)明确取值；反"AI 味"默认值(禁紫渐变/Inter/对称卡片堆砌/到处毛玻璃无目的) |
| design-taste-frontend | 设计读(Design Read)+三旋钮(本 UI: VARIANCE 6 / MOTION 5 / DENSITY 4)；**单一强调色锁定全页**；形状一致性锁定(按钮全胶囊/卡片 20px/开关系统默认)；反紫渐变、反 beige+brass 高级风 |
| impeccable | Operate 模式(完成任务优先)；有界打磨(一轮验证修复、不再无限打磨)；反 slop 纪律 |

## 四、架构决策（已定，勿改除非有充分理由）

- **模块 ID/名**：`libpool`（显示名 "LibPool · 库池"）。
- **管理工具**：`libman` —— Rust 写的本地 CLI（aarch64-unknown-linux-musl 纯静态，Rust std + serde_json 仅两个依赖，无 TLS 网络依赖）。
  - 所有下载调用模块自带的 `curl`；解压调用模块自带的 `tar/xz/zstd/unzip`（这些本身也是捆绑库）。这样 Rust 无需引入任何系统依赖，交叉编译极简、二进制小。
- **挂载机制**：
  - 模块 `system/bin/` 由 magisk/KSU magic mount 自动映射到 `/system/bin`（模块源在可写的 /data，故目标可写）。
  - `libman mount <id>`：确保目标存在后 `mount --bind <libfile> /system/bin/<bin>`（.so 挂到 /system/lib/<soname>），立即生效，无需重启。
  - `libman unmount <id>`：`umount` 已绑定的挂载。
  - `service.sh` 开机按 `state.json` 重放挂载（幂等）。
  - 用户开关后若 bind 被系统重启清空，重启后由 service.sh 恢复；UI 文案按"重启后可恢复，当前已立即生效"口径表述。
- **库来源（repos.json 三种 type）**：
  - `termux`：从 Termux 官方/镜像仓库 `Packages.xz` 解析最新版 → 下载 .deb → 提取 `usr/bin` 与 `usr/lib/*.so` 安装。自动跟最新版。
  - `url`：直接从给定 URL 下载已编译二进制（GitHub releases 等），需提供 `bins` 清单。
  - `bundled`：随模块安装/首次启动自动拉取的核心库（约 18 个，标记 `core: true`）。
- **KsuWebUI 通信**：KernelSU 管理器注入全局 `ksu` 对象；页面 `import { exec, toast, moduleInfo, fullScreen, enableEdgeToEdge } from './kernelsu.js'`（已内联官方 0 依赖源码）；非 KernelSU 环境(浏览器预览)自动降级 mock。
- **SELinux**：提供 `sepolicy.rule`，允许 `mount` 到 /system；脚本用 `MODDIR=${0%/*}` 取模块路径，禁止硬编码。

## 五、目录结构

```
libpool/
├── module.prop            # 元数据
├── customize.sh           # 安装：生成 core 库 symlink、初始化 state、后台装 core
├── service.sh             # 开机：libman apply 重放挂载 + 后台补装缺失 core
├── post-fs-data.sh        # 早期占位（轻量）
├── uninstall.sh           # 卸载：umount 全部 bind
├── sepolicy.rule
├── system/bin/            # 由 libman 生成 symlink/bind 目标占位（magic mount）
├── libs/<id>/bin|lib      # 实际库文件（bundled + 用户下载）
├── tools/libman           # Rust 静态二进制
├── webroot/               # KsuWebUI（index.html / style.css / app.js / kernelsu.js / data/）
│   └── data/state.json    # 启用状态（libman 读写）
├── repo_src/repos.json    # 300+ 扩展库数据库（构建时拷入 webroot/data/）
├── src/rust-libman/       # Rust 源码
├── scripts/build.ps1|sh   # 交叉编译脚本
└── README.md
```

## 六、设计读（Design Read，仅用于 UI）

> "Reading this as: Android 系统级库管理工具的 App Shell，面向动手能力强的技术用户（但文案要新手友好），Apple 简洁 + 通透毛玻璃语言，极简主义，Operate 模式，旋钮 6/5/4。"
> 强调色：亮色 #0071E3 / 暗色 #0A84FF（Apple 系统蓝，全页锁定）。背景：亮 #F5F5F7 / 暗 #000。卡片：rgba 白 + blur 20 + 1px 高光。
> v2 增补：G2 圆角（corner-shape: squircle，卡片 20px / 公告卡 26px）；触摸优先（涟漪 .ripple-host + 按压 :active scale 0.94 + 打断动画）；`@media (hover:hover)` 才放 hover 悬浮；主题切换加 `.theme-scrim` 丝滑遮罩；CSS 前同步脚本防 FOUC 闪白。

## 七、文案风格

中文、口语化、新手可懂。避免："/system", "bind mount", "overlayfs" 等术语直接甩给用户。用"已挂载 / 未挂载 / 下载中 / 精简掉"等日常词；专业词只作小字副注。

## 八、禁止事项

- 禁止紫渐变、禁止满屏毛玻璃无目的、禁止 emoji（除非用户要）、禁止 Inter 默认字体（用系统 SF 栈）。
- 禁止引入需要网络打包的依赖；webroot 必须完全离线可用。
- 禁止把未编译的占位当成品交付；libman 必须真正编译出 aarch64 静态二进制，且 shell fallback 脚本始终可用。
