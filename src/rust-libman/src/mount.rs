//! 挂载/卸载库到 /system/bin 和 /system/lib
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
//!
//! 挂载原理（重要）：
//!   LibPool 的本质是往 /system/bin 添加系统里原本没有的命令（adb、fastboot 等）。
//!   直接 bind mount 到 /system/bin/<name> 需要目标文件已存在，而 /system 只读、
//!   touch 建占位也会失败（这就是此前 "bind mount 失败: No such file or directory" 的根因）。
//!   因此正确做法是依赖 **Magisk / KernelSU 的 magic mount**：
//!     把库文件复制到模块自己的 system/bin、system/lib 目录，
//!     由宿主（Magisk/KernelSU/APatch）开机时把这些目录叠加（overlay）到系统 /system。
//!     这样不需要目标存在、不需要写 /system，新增命令开箱即用。
//!   对系统里已经存在的同名命令，再尝试 bind mount 做即时覆盖（bind 目标存在即可，立即生效）。
//!
//! 卸载时：从模块 system/ 目录移除文件（magic mount 叠加随即消失），并解除可能的 bind。
//! 所有操作幂等。

use crate::util;
use std::path::Path;

/// 挂载库：
/// 1) 把 bin/lib 复制到模块 system/bin、system/lib（magic mount 源，开机叠加到 /system）；
/// 2) 对系统已存在的同名命令尝试 bind mount 即时覆盖（失败不致命，由 magic mount 兜底）。
pub fn mount_lib(dir: &str, id: &str) -> Result<(), String> {
    let lib_root = format!("{dir}/libs/{id}");
    let sys_bin = format!("{dir}/system/bin");
    let sys_lib = format!("{dir}/system/lib");
    std::fs::create_dir_all(&sys_bin).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&sys_lib).map_err(|e| e.to_string())?;

    let bin_dir = format!("{lib_root}/bin");
    let lib_dir = format!("{lib_root}/lib");

    // 1) magic mount 源：复制到模块 system/（目标文件由宿主叠加进 /system）
    if Path::new(&bin_dir).is_dir() {
        for name in fs_entries(&bin_dir)? {
            let src = format!("{bin_dir}/{name}");
            let dst = format!("{sys_bin}/{name}");
            let _ = std::fs::copy(&src, &dst);
            let _ = util::run(&format!("chmod 755 '{dst}'"), None);
        }
    }
    if Path::new(&lib_dir).is_dir() {
        for name in fs_entries(&lib_dir)? {
            let src = format!("{lib_dir}/{name}");
            let dst = format!("{sys_lib}/{name}");
            let _ = std::fs::copy(&src, &dst);
        }
    }

    // 2) bind mount 即时覆盖：仅对系统已存在的同名目标生效（新命令由 magic mount 提供）
    if Path::new(&bin_dir).is_dir() {
        for name in fs_entries(&bin_dir)? {
            let src = format!("{bin_dir}/{name}");
            let dst = format!("/system/bin/{name}");
            if Path::new(&dst).exists() {
                let _ = bind_mount(&src, &dst);
            }
        }
    }
    if Path::new(&lib_dir).is_dir() {
        for name in fs_entries(&lib_dir)? {
            let src = format!("{lib_dir}/{name}");
            let dst = format!("/system/lib/{name}");
            if Path::new(&dst).exists() {
                let _ = bind_mount(&src, &dst);
            }
        }
    }

    println!("已挂载: {id}");
    Ok(())
}

fn fs_entries(dir: &str) -> Result<Vec<String>, String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .map(|r| r.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();
    v.sort();
    Ok(v)
}

/// bind mount（仅调用方确认目标已存在时使用；失败返回错误，由调用方决定是否忽略）。
fn bind_mount(src: &str, dst: &str) -> Result<(), String> {
    // 若目标已是 mount 点，先 umount 后重新 bind
    let (ok, out, _) = util::run(&format!("mount | grep ' on {dst} '"), Some(5))
        .unwrap_or((false, String::new(), String::new()));
    if ok && !out.trim().is_empty() {
        let _ = util::run(&format!("umount '{dst}'"), Some(10));
    }

    // 确保源存在
    if !Path::new(src).exists() {
        return Err(format!("源文件不存在: {src}"));
    }
    if !Path::new(dst).exists() {
        return Err(format!("目标不存在，由 magic mount 提供: {dst}"));
    }

    let (ok, _, err) = util::run(&format!("mount --bind '{src}' '{dst}'"), Some(15))?;
    if !ok {
        let (ok2, _, err2) = util::run(&format!("mount -o bind,noexec '{src}' '{dst}'"), Some(15))?;
        if !ok2 {
            return Err(format!("bind mount 失败: {err} {err2}"));
        }
    }
    Ok(())
}

/// 卸载指定库：
/// 1) 从模块 system/ 目录移除文件（magic mount 叠加随即消失）；
/// 2) 解除可能的 bind mount。
pub fn unmount_lib_by_id(dir: &str, id: &str) -> Result<(), String> {
    // 从 libs/<id>/meta.json 中读取 bins 和 libs
    let meta_path = format!("{dir}/libs/{id}/meta.json");
    if !Path::new(&meta_path).exists() {
        return Ok(()); // 未安装则视为已清理
    }
    let meta = crate::repo::get_meta(dir, id)?;

    for b in &meta.bins {
        let _ = std::fs::remove_file(format!("{dir}/system/bin/{b}"));
        let _ = util::run(&format!("umount '/system/bin/{b}'"), Some(10));
    }
    for l in &meta.libs {
        let _ = std::fs::remove_file(format!("{dir}/system/lib/{l}"));
        let _ = util::run(&format!("umount '/system/lib/{l}'"), Some(10));
    }
    println!("已卸载: {id}");
    Ok(())
}

/// 卸载所有已挂载的库
pub fn unmount_all(dir: &str) -> Result<(), String> {
    let ids = crate::repo::get_mounted_ids(dir)?;
    for id in &ids {
        let _ = unmount_lib_by_id(dir, id);
        crate::repo::mark_mounted(dir, id, false)?;
    }
    Ok(())
}
