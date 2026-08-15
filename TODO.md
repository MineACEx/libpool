# TODO — LibPool（库池）构建清单

## 阶段一：基础设施
- [x] 读取 4 个 skill（apple-ui-design / beautiful-ui / design-taste-frontend / impeccable）
- [x] 确认 KernelSU WebUI 通信方案（全局 ksu 对象 + kernelsu.js 内联）
- [x] 确认 Rust 交叉编译方案（aarch64-unknown-linux-musl + rust-lld，C 盘空间不足装不了 NDK）
- [x] 安装 Rust 工具链（rustup + stable 1.97.1 + aarch64-unknown-linux-musl target）

## 阶段二：模块本体
- [x] module.prop
- [x] customize.sh / service.sh / post-fs-data.sh / uninstall.sh / sepolicy.rule
- [x] Rust libman 源码（list/toggle/mount/unmount/install/remove/apply/status + core 自动安装）
- [x] 修复：移除缺失的 `mod state;`；Termux .deb 前缀路径解析（find_usr_root）
- [x] shell fallback：libman.sh（无二进制时兜底，sh/bash 语法校验通过）

## 阶段三：KsuWebUI
- [x] webroot/kernelsu.js（内联官方源码 + 浏览器 mock 降级）
- [x] webroot/index.html（三 Tab：已挂载 / 扩展库 / 设置；iOS18 毛玻璃）
- [x] webroot/style.css（亮暗切换 / 毛玻璃 / 响应式 720px 三列 / SF 栈 / 单一强调色）
- [x] webroot/app.js（exec 通信 / 搜索 / 分类 / 开关 / 下载进度 / 精简）
- [x] webroot/data/state.json（初始状态，repo_src/state.json 提供）

## 阶段四：扩展库数据库
- [x] repo_src/repos.json：**136 个**扩展库（termux/url/bundled 三种来源，10 大分类，18 个核心库）

## 阶段五：构建与交付
- [x] scripts/build.ps1 + build.sh（交叉编译 + 组装）
- [x] 编译 libman aarch64 静态二进制（0 告警 0 错误）
- [x] 打包 libpool-<ver>.zip（287KB，可直接刷入）
- [x] README.md（安装/使用/常见问题）
- [x] 最终自查（UI 双主题 / 平板 / 对比度 / 动效克制）
  - [x] kernelsu.js 修复 isMock 未声明 bug（主副本 + module 副本同步）
  - [x] Rust 源码清理：删除未用函数（is_elf/is_elf64/get_bins/Json::arr/s/b/n）+ 未用参数，0 告警
  - [x] 浏览器实测：亮/暗主题切换正常（#000 暗色 / #F2F2F7 亮色）
  - [x] 响应式：360px 单列、720px+ 三列、平板 1024px 无水平滚动
  - [x] 修复水平滚动：html/body overflow-x:hidden + 小屏 480px 断点单列
  - [x] 毛玻璃：卡片 backdrop-filter blur(24px) 实测生效

## 待验证
- [ ] libman 二进制 `list` 输出与 WebUI 兼容（需真机）
- [ ] customize.sh / service.sh 在真机流程正确（需真机）

## 阶段六：v2 增强（触摸优先 + G2 + 公告 + 背景 + 品牌）
- [x] G2 连续圆角：卡片 `border-radius + corner-shape: squircle`（Chrome 139+/WebView，渐进增强）
- [x] 触摸优先动效：点击涟漪（.ripple-host + pointerdown）、按压 :active scale(0.94)、打断动画（快速连点涟漪叠加扩散）
- [x] hover 改为 `@media (hover: hover)` 桌面增强；触控目标 ≥44px
- [x] 主题防闪白（FOUC）：head 内 CSS 前同步脚本应用已存主题
- [x] 假进度修复：未知进度改不确定光条（indeterminate 动画）
- [x] 下载遮罩加「取消下载」入口（pkill libman install）
- [x] 搜索防抖 150ms + 重建跳过入场动画（首屏才播）
- [x] 镜像选择 UI 重构：毛玻璃单选卡片（role=radiogroup + ripple-host）
- [x] 自定义背景图：设置页 URL → `.custom-bg` 层 + localStorage `libpool-bg`，毛玻璃遮罩在卡片之上
- [x] 上下边框滚动模糊过渡：body.scrolled → .top-blur/.bottom-blur 渐变
- [x] 公告功能：毛玻璃卡片 + 背景 backdrop-filter 0→22px 丝滑渐变（仿 iOS 开合）+ 每天一次 + 无障碍
- [x] 库数据库：136 → **300** 个扩展库（10 分类），18 个核心自带库（curl/git/tar/unzip/awk 等基础库）
- [x] 品牌：module.prop author = MINO · Himer；顶部 tagline「by MINO」；图标 webroot/icons/icon.png（用户源 PNG，804KB）
- [x] 重打包 dist/libpool-1.0.0.zip（1.09MB，含新图标）
- [x] 浏览器验证：300 库渲染、G2 squircle 生效、tab/主题/搜索/镜像/涟漪/公告模糊过渡/自定义背景全部正常、无控制台报错、无水平溢出

## 待办（用户提供后完成）
- [ ] **公告网址**：用户可在模块目录 `/.git/url.txt` 手动填写纯文本网址（无需改代码），或在 app.js `ANNOUNCEMENT_URL` 填默认值
- [ ] 真机验证 libman list / 安装流程 / WebUI 通信 / 32 位设备（KSU 真机）

## 阶段七：v3 优化（功能安全 + 32 位 + 实用库 + UI 精修 + 许可）
- [x] **公告 .git/url.txt**：模块根 `/.git/url.txt` 可手动编辑；app.js resolveAnnounceUrl 优先读取（exec cat），回退 ANNOUNCEMENT_URL
- [x] **安全加固**：customize.sh 秒回 + 后台 core；service.sh apply timeout 60 + ensure-core 后台 timeout 600；post-fs-data 仅 mkdir；uninstall 容错 timeout 60；全部幂等
- [x] **占用≈0**：无守护进程/无轮询/无自启常驻，按需运行即退（README 声明）
- [x] **32 位兼容**：编译 armv7-unknown-linux-musleabihf → tools/libman-arm（495KB）；脚本按 uname -m 自动选择 原生/arm/shell 三件套
- [x] **实用库扩充**：322 库（新增 android-tools/adb/fastboot、iproute2、pciutils、usbutils、sysstat、smartmontools、hdparm、fdupes、cpulimit、bc、expect、which、hping3、scala、dlang、ghostscript、qpdf、tesseract 等 22 个）
- [x] **卡片圆角开放**：统一 24px（公告 30px），corner-shape: squircle
- [x] **间距留白**：gap/padding 全面加大（grid 14px、card 18-20px、margin 16px+）
- [x] **打断动画修复**：fadeUp 改 animation-fill-mode: backwards（解决 :active 按压被 animation 锁定失效的真 bug）；store/lib-card :active scale 生效
- [x] **并行动画**：多属性 transition 并行；涟漪叠加打断；主题遮罩渐变+高斯模糊+先快后慢曲线（--ease-out）
- [x] **极窄对等边框**：--card-border-fine（亮 rgba(0,0,0,.10) / 暗 rgba(255,255,255,.14)）统一 1px
- [x] **本地相册背景**：btnPickBg + input[type=file] + canvas 压缩为 JPEG DataURL（最长边 1600 / 质量 0.8）存 localStorage；文字亮暗自由切换保留
- [x] **主题切换渐变模糊遮罩**：theme-scrim 径向渐变 orb + backdrop blur 18px + 先快后慢
- [x] **toast 亮暗双色 + 毛玻璃 + G2**：24px 大圆角矩形 + squircle + --toast-bg/--toast-text/--toast-error-bg 变量 + .error 类
- [x] **TAB 轻微回弹**：tab-indicator cubic-bezier(0.34,1.4,0.5,1) 0.5s（克制超调）
- [x] **图标去描边遮罩**：brand-icon 移除 border/inset shadow，仅 G2 圆角（22px squircle）+ mask 裁剪 PNG
- [x] **滚动入场**：.reveal + IntersectionObserver（不监听 scroll）+ 面板切换 revealRecheck 强制重检；先快后慢短时长 + --i 错峰
- [x] **加宽布局**：.app max-width 760→880px、padding 16→24px（平板 32px），侧边光效不再硬切
- [x] **修复页面无法滚动**：移除 html/body height:100%（锁定文档高度 bug）
- [x] **README 用 Rust 干了什么**：命令职责表 + 为何 Rust + 32 位表 + 公告配置
- [x] **Apache-2.0 + 作者声明**：全部源码/脚本/构建/Rust 文件头 + README/module.prop
- [x] 重打包 dist/libpool-1.0.0.zip（1.34MB，含 .git/url.txt + 双架构二进制 + 322 库）
- [x] 浏览器验证：322 库渲染、滚动入场逐卡滑入（4→12→20）、toast 亮暗双色毛玻璃 G2、tab 回弹曲线、主题渐变模糊遮罩、无控制台报错、无水平溢出

## 阶段八：v4 UI 精修（本轮，用户 9 点意见）
- [x] **圆角统一 32px + G2**：--r-card=32px 全面覆盖卡片/下载按钮/primary/ghost/danger 按钮/toast/hint-card；修复小屏断点 store-card 24px 回退；mirror-input 28px 与搜索框一致；全部 corner-shape: squircle
- [x] **页面切换运动模糊**：.page-transition 遮罩 0→24px blur 丝滑拉起（0.32s --ease-ios），面板互换后回落；pointer-events:none；连续点击强制回流重触发
- [x] **TAB 动画柔化**：tab-indicator 改 cubic-bezier(0.22,1,0.36,1) 0.5s（平滑减速无超调，去掉旧 1.4 超调僵硬）
- [x] **透明度/模糊度设置滑杆**：--card-alpha/--card-blur CSS 变量 + 设置页两滑杆实时写 + localStorage 持久化（alpha 20-100% / blur 0-40px）
- [x] **滚轮滚动修复**：overflow-x: hidden → clip（不建滚动容器），@supports 回退；移除 height:100% 用 min-height:100dvh；桌面/移动实测可滚
- [x] **顶部模糊层去描边 + 渐变模糊**：.top-blur 去 border-bottom，mask-image linear-gradient（顶 100% → 55% 处 35% → 底 0）丝滑过渡
- [x] **toast 进出场动画**：默认 translateY(14px)+opacity 0 → .visible 归位淡入；退场走反向过渡再 hidden；连续 toast 打断平滑收尾
- [x] **移动端动效**：背景光斑 orbFloat 漂浮、统计数字 count-up（easeOutCubic）、安装成功 installPulse 光晕；全部纯 transform/opacity、prefers-reduced-motion 统一关闭、零鼠标依赖
- [x] 同步 module/webroot 副本（style/app/index/kernelsu 逐字节一致）
- [x] 重打包 dist/libpool-1.0.0.zip（1.34MB，zip 条目修正为标准正斜杠路径）
- [x] 浏览器实测（Chrome DevTools）：桌面 1280 滚动成功 + 扩展库 21327px 可滚；360 移动单列 32px 圆角；1024 平板三列；暗色 #000 生效；滑杆写变量 + localStorage；toast 进出场；页面切换 blur 过渡；控制台无报错

## 阶段九：v5 bug 修复（用户 7 点意见，真机实测前最后一轮）
- [x] **上下模糊全局应用 + 模糊不消失**：top/bottom-blur 改用主题底色固定 74%/78% + 固定 blur 18px/14px（不再随卡片透明度/模糊滑杆联动），mask 渐变放宽（0→40%→72%→100%），任何设置下都全局可见
- [x] **圆角 31→35px**：--r-card 35px / --r-input 31px / --r-sm 20px；按钮/toast/hint/输入框全同步
- [x] **下载按钮"缺块"根因修复**：移除 .ripple-host 与 .brand-icon 的 `mask-image: radial-gradient(white,black)`（该 mask 会把元素自身边缘淡出裁掉，是缺块 + 圆角锯齿的元凶）；涟漪裁剪仅靠 overflow:hidden
- [x] **暗色卡片上边框等宽**：移除全部卡片/输入框/toast/公告卡的 `inset 0 1px 0 rgba(255,255,255,0.28)` 顶部高光，统一 `border: 1px solid var(--card-border-fine)` 四边等宽
- [x] **KSUWEBUI 配置可保存**：镜像选中态 + 自定义地址持久化到 localStorage（libpool-mirror / libpool-mirror-custom），预览/浏览器也生效，真机仍写模块
- [x] **页面/主题切换模糊强度对调**：页面切换遮罩 24px→4px（几乎不可见、仔细看才察觉）；主题切换遮罩 18px→36px（强烈毛玻璃过渡）
- [x] **加载边滑边加载 + 图标 1:1**：reveal 的 IO 改 threshold 0.01 + rootMargin 0（进入视口立即触发）；stagger 延迟 45ms→20ms；.lib-icon 加 aspect-ratio:1/1 + flex:none，store 图标统一 36px（移除内联尺寸）
- [x] **G2 平滑度可调**：设置页新增"圆角平滑度"滑杆（0-100，默认 100），映射 --radius-shape（≥50=squircle G2 / <50=round 经典抗锯齿），localStorage 持久化
- [x] 同步 module/webroot 副本 + 重打包 dist/libpool-1.0.0.zip（1.34MB，正斜杠）
- [x] 浏览器实测：35px 圆角、btnMask none、平滑度滑杆 round/squircle 切换、镜像持久化、等宽边框、全局模糊条（暗色 topBg 74% + blur 18px）、页面切换 blur 3.7px 过渡中间值、主题切换规则 blur 36px

## 待真机验证
- [ ] libman 二进制 `list` 输出与 WebUI 兼容（需真机）
- [ ] customize.sh / service.sh 在真机流程正确（需真机）
- [ ] KSU WebUI 配置保存（镜像写模块 + localStorage 双保险）

## 阶段十：v6 功能增强 + 渲染修复（用户 7 点意见）
- [x] **圆角控制重构**：G2/经典改为切换按钮（seg），另加"圆角大小"滑杆（16-48px 默认 32 中间位），两种模式都生效并持久化
- [x] **下载按钮缺三角根因修复**：按钮用独立 `--r-btn: min(calc(var(--r-card)-6px), 22px)`（自动适配小尺寸）+ 不套 squircle（需 overflow:hidden 裁涟漪，与超椭圆边界冲突）；卡片保持 G2 squircle。实测按钮 22px 经典圆角、无缺口
- [x] **主题切换模糊丝滑**：scrim.on 保持 80ms→340ms（blur 过渡 0.55s 充分升起可见再回落）；opacity 0.3s 协调
- [x] **上下遮罩渐变**：bottom-blur 补 mask（to top 渐变，底部完全模糊→向上淡出，与 top-blur 对称）；top/bottom 均独立全局毛玻璃（blur 18/14px + 主题底色 74%/82%）
- [x] **搜索框文本位置**：左 padding 46→40px（文本左移）、上下 12px 等距、图标 left 14px、背景改 --card-bg-strong（更不透明）
- [x] **扩展库加载动画恢复**：reveal 入场距离 18→24px + scale(0.99)、时长 0.5s/0.45s、delay 40ms（上限 12）、IO threshold 0（进入视口立即触发，边滑边加载）
- [x] **按钮独立透明度/模糊**：新增 --btn-alpha/--btn-blur 变量 + 设置页两滑杆；下载/primary/ghost/danger 按钮改半透明毛玻璃（color-mix + backdrop-filter）
- [x] 同步 module/webroot + 重打包 dist/libpool-1.0.0.zip（1.34MB，正斜杠）
- [x] 浏览器实测：按钮 22px 经典圆角 + blur18 + 透明度 92%（可调 60%）；卡片 G2 32px；大小滑杆 20/44 双模式生效；bottom-blur mask 渐变；搜索框 padding 12/16/12/40；主题遮罩 340ms 保持

## 阶段十一：v7 统一圆角体系 + 动画/遮罩修复（用户 7 点意见）
- [x] **全组件统一圆角 + 全部 G2**：--r-btn/--r-input/--r-sm 全部改为 --r-card 比例派生（JS 计算避免 Blink min/max 嵌套 calc 重算 bug），滑杆一动全盘同步；按钮恢复 corner-shape: squircle（G2），圆角封顶 20px 防小按钮变形
- [x] **扩展库动画可见**：IO threshold 0→0.12（元素进入 12% 才触发，避免动画在用户注意到前完成）+ 位移 24→28px + 时长 0.5→0.55s
- [x] **圆角类型切换 UI 对齐**：radiusSeg 改为与主题切换一致的行内右对齐（setting-row 内），按钮尺寸按内容自适应
- [x] **去除页面切换模糊遮罩**：移除 .page-transition 元素/CSS/JS 逻辑，切 Tab 仅保留指示条滑动 + 面板切换
- [x] **主题切换模糊修复**：scrim 背景纯色→半透明 70% + 峰值 opacity 0.9（blur 透出、页面始终可见，不再"模糊丢失/中途不可见"）
- [x] **上下渐变模糊修复**：top-blur 背景 74%→45%、bottom-blur 82%→52%（低不透明度让 blur 18/14px 渐变真正透出，不再被纯色背景盖住）
- [x] 同步 module/webroot + 重打包 dist/libpool-1.0.0.zip（1.34MB，正斜杠）
- [x] 浏览器实测：按钮 G2 squircle 跟随滑杆（16→12px / 44→20px 封顶）、输入框 38px/图标 22px、卡片 44px；scrim 半透明 70%；page-transition 已移除；radiusSeg 行内布局一致

## 阶段十二：v8 收尾（增强模糊 + 纯模糊主题过渡 + 返回顶部键）
- [x] **上下过渡模糊增强（不固定死）**：top/bottom-blur 的 blur 改跟随卡片模糊滑杆（var(--card-blur)，默认 28px 更强），背景不透明度 55%/60% 让模糊明显透出
- [x] **模块图标去描边/去底图遮罩**：brand-icon 移除 box-shadow、overflow:hidden、border-radius 裁剪，img 改 object-fit: contain 完整呈现
- [x] **TAB 栏对比度**：亮色 tabbar 0.6→0.72、暗色 0.55→0.68
- [x] **主题切换纯模糊**：scrim 背景改全透明（透明度 0 的纯模糊），仅 backdrop-filter blur(36px) 全局模糊整个界面；曲线用先快后慢 --ease-out 0.5s
- [x] **返回顶部毛玻璃键**：滚动进度超 15% 时从底部丝滑弹出（translateY 24→0 + opacity + scale，先快后慢）；点击 smooth 回顶；透明度/模糊绑定按钮滑杆（--btn-alpha/--btn-blur）；48px 触摸目标 + 按压反馈
- [x] 同步 module/webroot + 重打包 dist/libpool-1.0.0.zip（1.34MB，正斜杠）
- [x] 浏览器实测：scrim 背景 rgba(0,0,0,0) 纯透明 + blur36 规则；top-blur 跟随 --card-blur；brand-icon 无 shadow/overflow + contain；tabbar 0.68；返回键 15% 弹出/回顶隐藏/绑定 btn-alpha 0.55 + blur 18px
