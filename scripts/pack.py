#!/usr/bin/env python3
# =============================================================================
# LibPool · 库池 — pack.py（打包脚本）
# Copyright (C) 2026 MINO · Himer (MineACE)
# SPDX-License-Identifier: Apache-2.0
#
# 把模块目录打包成 KernelSU/Magisk 可刷入的 zip。
# 关键：条目路径一律使用正斜杠 '/'。
#   - Windows .NET ZipFile 可能产出反斜杠 '\\' 条目，设备端 unzip 会把它当成
#     一个字面文件名（如 webroot\\style.css），导致模块结构损坏。
#   - 本脚本用 zipfile 显式把 os.sep 替换为 '/'，保证任何平台打包都正确。
# 用法：python pack.py <模块目录> <输出zip>
# =============================================================================
import os
import sys
import zipfile


def pack(src: str, out: str) -> None:
    src = os.path.abspath(src)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for root, dirs, files in os.walk(src):
            # 空目录也写入条目（magic mount 目标 system/bin、system/lib 等需要存在）
            for d in dirs:
                full = os.path.join(root, d)
                rel = os.path.relpath(full, src).replace(os.sep, "/")
                z.writestr(rel + "/", "")
            for f in files:
                full = os.path.join(root, f)
                rel = os.path.relpath(full, src).replace(os.sep, "/")
                z.write(full, rel)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(__doc__)
        sys.exit(1)
    pack(sys.argv[1], sys.argv[2])
    print(f"packed: {os.path.abspath(sys.argv[2])}")
