//! 挂载/卸载库到 /system/bin 和 /system/lib
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
//!
//! 挂载原理（重要）：
//!   依赖 Magisk / KernelSU / APatch 的 magic mount：把库文件复制进模块的
//!   system/bin、system/lib 目录，由宿主开机叠加（overlay）到系统 /system。
//!
//! 安全红线（血的教训）：
//!   - **绝不覆盖系统已有的同名文件**。尤其不要拿 Termux 的 .so（如
//!     libcrypto.so / libssl.so）叠加到 /system/lib 去覆盖系统原生库——那会让
//!     依赖这些库的系统应用全部崩溃（症状：软件打不开、提示不兼容）。
//!   - 因此只"新增系统没有的文件"；系统已存在的同名文件一律跳过。
//!   - 不用 bind mount（bind 会覆盖目标，同样危险）。
//!   副作用：无法用模块命令覆盖系统同名命令（安全优先，可接受）。
//!
//! 卸载时：删除模块 system/ 目录里我们自己添加的文件（系统同名的一开始就没复制），
//! 并顺手清理旧版本可能遗留的 bind mount。所有操作幂等。

use crate::util;
use std::path::Path;

/// 挂载库：把 bin/lib 中"系统里不存在同名"的文件复制进模块 system/（magic mount 源）。
pub fn mount_lib(dir: &str, id: &str) -> Result<(), String> {
    let lib_root = format!("{dir}/libs/{id}");
    let sys_bin = format!("{dir}/system/bin");
    let sys_lib = format!("{dir}/system/lib");
    std::fs::create_dir_all(&sys_bin).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&sys_lib).map_err(|e| e.to_string())?;

    let bin_dir = format!("{lib_root}/bin");
    let lib_dir = format!("{lib_root}/lib");

    if Path::new(&bin_dir).is_dir() {
        for name in fs_entries(&bin_dir)? {
            copy_if_safe(dir, id, &bin_dir, &name, &sys_bin, "/system/bin");
        }
    }
    if Path::new(&lib_dir).is_dir() {
        for name in fs_entries(&lib_dir)? {
            copy_if_safe(dir, id, &lib_dir, &name, &sys_lib, "/system/lib");
        }
    }

    println!("已挂载: {id}");
    Ok(())
}

/// 仅当系统没有同名文件（且模块里还没有自己的副本）时才复制。
/// 模块里已有自己的副本 → 覆盖更新（幂等重挂载）；系统已有同名 → 跳过，绝不覆盖。
fn copy_if_safe(
    dir: &str,
    id: &str,
    src_dir: &str,
    name: &str,
    mod_dir: &str,
    sys_dir: &str,
) {
    let src = format!("{src_dir}/{name}");
    let mod_path = format!("{mod_dir}/{name}");
    let sys_path = format!("{sys_dir}/{name}");

    if !Path::new(&mod_path).exists() && Path::new(&sys_path).exists() {
        util::log_file(
            dir,
            &format!("{id}: 跳过 {name}（{sys_path} 已存在，不覆盖）"),
        );
        return;
    }
    let _ = std::fs::copy(&src, &mod_path);
    let _ = util::run(&format!("chmod 755 '{mod_path}'"), None);
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

/// 卸载指定库：删除模块 system/ 里我们自己添加的文件，并清理可能残留的 bind mount。
pub fn unmount_lib_by_id(dir: &str, id: &str) -> Result<(), String> {
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
