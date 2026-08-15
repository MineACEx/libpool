#!/usr/bin/env python3
# =============================================================================
# LibPool · 库池 — assemble.py（组装 + 打包脚本）
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
#
# 组装模块目录并打包成 KernelSU/Magisk 可刷入的 zip。
# 前置：aarch64 原生二进制已交叉编译到
#       src/rust-libman/target/aarch64-unknown-linux-musl/release/libman，
#       armv7 二进制已放到 tools/libman-arm（可选）。
# 打包条目路径一律正斜杠（见 pack.py），杜绝设备端出现反斜杠文件名。
# 用法：python scripts/assemble.py
# =============================================================================
import os
import re
import shutil
import sys

# 让脚本可从任意目录运行：以本文件所在目录的上级为项目根
ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))


def main() -> int:
    mod = os.path.join(ROOT, "module")
    if os.path.exists(mod):
        shutil.rmtree(mod)
    os.makedirs(mod)

    # 1) 模块脚本与元数据
    for f in ["module.prop", "customize.sh", "service.sh", "post-fs-data.sh", "uninstall.sh", "sepolicy.rule"]:
        shutil.copy2(os.path.join(ROOT, f), os.path.join(mod, f))

    # 2) WebUI 与仓库数据
    shutil.copytree(os.path.join(ROOT, "webroot"), os.path.join(mod, "webroot"))
    os.makedirs(os.path.join(mod, "webroot", "data"), exist_ok=True)
    shutil.copy2(os.path.join(ROOT, "repo_src", "repos.json"), os.path.join(mod, "webroot", "data", "repos.json"))
    state = os.path.join(ROOT, "repo_src", "state.json")
    if os.path.exists(state):
        shutil.copy2(state, os.path.join(mod, "webroot", "data", "state.json"))

    # 3) 管理工具（原生优先 + shell 兜底）
    os.makedirs(os.path.join(mod, "tools"), exist_ok=True)
    native = os.path.join(ROOT, "src", "rust-libman", "target", "aarch64-unknown-linux-musl", "release", "libman")
    if os.path.exists(native):
        shutil.copy2(native, os.path.join(mod, "tools", "libman"))
    else:
        print("警告：未找到 aarch64 二进制，请先交叉编译（tools/libman 缺失）", file=sys.stderr)
    arm = os.path.join(ROOT, "tools", "libman-arm")
    if os.path.exists(arm):
        shutil.copy2(arm, os.path.join(mod, "tools", "libman-arm"))
    shutil.copy2(os.path.join(ROOT, "tools", "libman.sh"), os.path.join(mod, "tools", "libman.sh"))

    # 4) 公告/云更新配置（模块内名为 .git/，避免与 git 仓库冲突）
    os.makedirs(os.path.join(mod, ".git"), exist_ok=True)
    for f in ["url.txt", "update.txt", "download.txt"]:
        s = os.path.join(ROOT, "module-config", f)
        if os.path.exists(s):
            shutil.copy2(s, os.path.join(mod, ".git", f))

    # 5) magic mount 目标空目录
    for d in [["system", "bin"], ["system", "lib"], ["libs"]]:
        os.makedirs(os.path.join(mod, *d), exist_ok=True)

    # 6) 打包
    ver = re.search(r"^version=(.+)$", open(os.path.join(ROOT, "module.prop"), encoding="utf-8").read(), re.M)
    ver = ver.group(1).strip() if ver else "0.0.0"
    os.makedirs(os.path.join(ROOT, "dist"), exist_ok=True)
    zpath = os.path.join(ROOT, "dist", f"libpool-{ver}.zip")
    if os.path.exists(zpath):
        os.remove(zpath)

    # 复用 pack.py 的打包逻辑（正斜杠条目）
    sys.path.insert(0, os.path.join(ROOT, "scripts"))
    import pack
    pack.pack(mod, zpath)
    print(f"打包完成: {zpath}")
    print(f"模块目录: {mod}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
