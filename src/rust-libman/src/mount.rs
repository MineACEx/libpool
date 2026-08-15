//! 挂载/卸载库到 /system/bin 和 /system/lib
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
//! 使用 mount --bind 实现即时生效，无需重启。
//! 卸载时 umount 即可。
//! 所有操作幂等。

use crate::util;

/// 挂载库：将 libs/<id>/bin/* → /system/bin/ 的 bind mount
///         将 libs/<id>/lib/* → /system/lib/ 的 bind mount
pub fn mount_lib(dir: &str, id: &str) -> Result<(), String> {
    // 确保挂载目录可写（magic mount 作用下 /system/bin 实际可写）
    // 对于不需要 bind 的单文件，直接用 copy 更可靠——但用户要求"挂载"，
    // 使用 bind mount 实现即时生效
    let lib_root = format!("{dir}/libs/{id}");

    // 挂载 bin
    let bin_dir = format!("{lib_root}/bin");
    if std::path::Path::new(&bin_dir).is_dir() {
        let entries = fs_entries(&bin_dir)?;
        for name in &entries {
            // 部分文件可能已通过 post-fs-data 等直接放在 system/bin 中
            let src = format!("{bin_dir}/{name}");
            let dst = format!("/system/bin/{name}");
            bind_mount(&src, &dst)?;
        }
    }

    // 挂载 lib
    let lib_dir = format!("{lib_root}/lib");
    if std::path::Path::new(&lib_dir).is_dir() {
        let entries = fs_entries(&lib_dir)?;
        for name in &entries {
            let src = format!("{lib_dir}/{name}");
            let dst = format!("/system/lib/{name}");
            let _ = bind_mount(&src, &dst); // 非致命：lib 挂载失败可能因 lib 不存在
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

fn bind_mount(src: &str, dst: &str) -> Result<(), String> {
    // 确保目标父目录存在
    let parent = std::path::Path::new(dst).parent().unwrap_or(std::path::Path::new("/"));
    let _ = std::fs::create_dir_all(parent);

    // 若目标已是 mount 点，先 umount 后重新 bind
    // 检查是否已挂载：mount | grep dst
    let (ok, out, _) = util::run(&format!("mount | grep ' on {dst} '"), Some(5)).unwrap_or((false, String::new(), String::new()));
    if ok && !out.trim().is_empty() {
        let _ = util::run(&format!("umount '{dst}'"), Some(10));
    }

    // 确保源存在
    if !std::path::Path::new(src).exists() {
        return Err(format!("源文件不存在: {src}"));
    }

    // 如果目标不存在，用 touch 创建占位
    if !std::path::Path::new(dst).exists() {
        let (ok, _, _) = util::run(&format!("touch '{dst}'"), Some(5))?;
        if !ok {
            // 创建失败可能是权限问题，尝试直接 bind（某些 KSU 内核允许）
            eprintln!("创建占位失败: {dst}，尝试直接 bind mount");
        }
    }

    // 执行 bind mount
    let (ok, _, err) = util::run(&format!("mount --bind '{src}' '{dst}'"), Some(15))?;
    if !ok {
        // 尝试 noexec 降级参数
        let (ok2, _, err2) = util::run(&format!("mount -o bind,noexec '{src}' '{dst}'"), Some(15))?;
        if !ok2 {
            return Err(format!("bind mount 失败: {err} {err2}"));
        }
    }
    Ok(())
}

/// 卸载指定库的所有 bind mount
pub fn unmount_lib_by_id(dir: &str, id: &str) -> Result<(), String> {
    // 从 libs/<id>/meta.json 中读取 bins 和 libs
    let meta_path = format!("{dir}/libs/{id}/meta.json");
    if !std::path::Path::new(&meta_path).exists() {
        return Ok(()); // 未安装则视为已清理
    }
    let meta = crate::repo::get_meta(dir, id)?;

    for b in &meta.bins {
        let dst = format!("/system/bin/{b}");
        let _ = util::run(&format!("umount '{dst}'"), Some(10));
    }
    for l in &meta.libs {
        let dst = format!("/system/lib/{l}");
        let _ = util::run(&format!("umount '{dst}'"), Some(10));
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