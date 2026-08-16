# 欢迎使用 LibPool v1.0.0

感谢你选择 **LibPool（库池）**！这是一个为 Android 用户补齐系统缺失库的模块，支持 **Magisk / KernelSU / APatch**，兼容 32 位设备。

## 它能做什么

- **233 个扩展库**，10 大分类：adb / fastboot、curl、git、htop、strace、tmux、rsync、jq、ffmpeg、nodejs、go、python 等，一键下载
- **18 个核心库** 开机自动就位，开箱即用
- **标准 magic mount**：库文件写入模块 system/ 目录，由 Magisk / KernelSU / APatch 开机叠加到 /system，新增命令开箱即用；系统已有同名命令则 bind mount 即时覆盖
- **挂载即时生效、无需重启**：mount / unmount 走 overlayfs 热挂载，命令执行完库立刻可用；开机自动重放配置，重启不丢
- **系统已有库绝不覆盖**：挂载前先检查 /system 里是否已存在同名 bin / lib，存在则跳过，绝不覆盖系统原生文件
- **按应用深度隐藏 root**：在「深度隐藏」页添加手机里已装的应用（真实图标），加进 KernelSU denylist 彻底隐藏 su 痕迹，开关即时生效
- **统一日志**：安装 / 服务 / WebUI / 工具日志全部整齐存放在模块目录 log/ 文件夹，方便问题追溯
- **低功耗**：核心逻辑用 Rust 原生二进制实现，无守护进程、无后台常驻，空闲占用趋近 0

## 怎么用

1. 打开 KernelSU 管理器 → 模块 → LibPool → WebUI
2. 在「扩展库」页搜索你需要的库，点下载
3. 在「已挂载」页一键开关、删除精简
4. 在「深度隐藏」页添加需要隐藏 root 的应用（银行 / MOMO / 淘宝等），开关即时生效

> 大陆网络建议在「设置 → 下载镜像」切换清华或北外镜像，下载更稳。
>
> 安装或更新模块后建议**重启一次**，让开机脚本重放挂载并补装核心库。

## 遇到问题？

欢迎到 [GitHub Issues](https://github.com/MineACEx/libpool/issues) 反馈，请附上机型、安卓版本、架构（uname -m）和模块版本号。

祝你用得顺手！
