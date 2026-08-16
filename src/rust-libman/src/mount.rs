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

// ---------------------------------------------------------------------------
// 热挂载：无需重启即可生效（安全优先：绝不覆盖系统已有同名文件）
// ---------------------------------------------------------------------------

/// 检查 /proc/self/mountinfo，判断 <sys_dir> 上是否已存在 overlay 挂载。
fn overlay_is_mounted(sys_dir: &str) -> bool {
    if let Ok(text) = std::fs::read_to_string("/proc/self/mountinfo") {
        for line in text.lines() {
            if !line.contains("overlay") {
                continue;
            }
            // mountinfo 第 5 个字段是挂载点（索引 0..=4），精确匹配避免子串误判
            let mut fields = line.split_whitespace();
            let _ = fields.nth(3); // 跳到索引 3（root）
            if let Some(mp) = fields.next() {
                if mp == sys_dir {
                    return true;
                }
            }
        }
    }
    false
}

/// 用 overlayfs 把模块 system/<dir> 叠加到系统 <sys_dir>，upper = 模块 system 目录。
/// 之后写入模块 system/ 的文件立即在系统路径可见；重启后仍由 magic mount 延续同一份文件。
///
/// 安全前提：模块 system 目录只允许包含「系统里不存在」的文件（由 copy_if_safe 保证），
/// 因此 overlay 只会新增文件，绝不会覆盖系统同名文件。
/// 返回是否已处于挂载状态（本次挂载成功或此前已挂载都算）。
fn overlay_hot_mount(dir: &str, sys_dir: &str, mod_dir: &str, work_dir: &str) -> bool {
    if overlay_is_mounted(sys_dir) {
        return true; // 已经热挂载过（不依赖标记文件，重启后也能正确判断）
    }
    let _ = std::fs::create_dir_all(work_dir);
    let _ = std::fs::create_dir_all(mod_dir);
    let cmd = format!(
        "mount -t overlay overlay -o lowerdir={sys_dir},upperdir={mod_dir},workdir={work_dir} {sys_dir}"
    );
    match util::run(&cmd, Some(20)) {
        Ok((true, _, _)) => {
            util::log_file(dir, &format!("热挂载成功: overlay {sys_dir} -> {mod_dir}"));
            true
        }
        Ok((false, _, err)) => {
            // 命令报错但已处于 overlay 状态（如并发/已被挂载）也视为成功
            if overlay_is_mounted(sys_dir) {
                true
            } else {
                util::log_file(dir, &format!("热挂载失败(overlay {sys_dir}): {err}"));
                false
            }
        }
        Err(e) => {
            util::log_file(dir, &format!("热挂载失败(overlay {sys_dir}): {e}"));
            false
        }
    }
}

/// 安全兜底：清除模块 system/ 里「会覆盖系统同名文件」的历史残留。
/// 正常流程 copy_if_safe 只复制系统里不存在的文件；这里清理旧版本可能遗留的冲突项，
/// 保证 overlay 只做新增、绝不覆盖系统文件。当前库自己的文件不清理。
fn prune_system_conflicts(dir: &str, id: &str) -> usize {
    use std::collections::HashSet;
    let mut removed = 0usize;
    let owned: HashSet<String> = match crate::repo::get_meta(dir, id) {
        Ok(m) => {
            let mut s = HashSet::new();
            for b in &m.bins {
                s.insert(format!("bin/{b}"));
            }
            for l in &m.libs {
                s.insert(format!("lib/{l}"));
            }
            s
        }
        Err(_) => HashSet::new(),
    };
    for (sub, sys_sub) in [("bin", "/system/bin"), ("lib", "/system/lib")] {
        let mod_sub = format!("{dir}/system/{sub}");
        let Ok(rd) = std::fs::read_dir(&mod_sub) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let rel = format!("{sub}/{name}");
            if owned.contains(&rel) {
                continue; // 当前库的合法文件
            }
            if Path::new(&format!("{sys_sub}/{name}")).exists() {
                let _ = std::fs::remove_file(format!("{mod_sub}/{name}"));
                util::log_file(dir, &format!("安全清理冲突残留: {sys_sub}/{name}"));
                removed += 1;
            }
        }
    }
    removed
}

/// 挂载库并尽量即时生效（无需重启），同时保证绝对安全：
///   1) 复制进模块 system/（**系统已有同名 bin/lib 一律跳过、绝不覆盖**；重启后 magic mount 兜底）；
///   2) 清理历史残留的、会覆盖系统文件的冲突项；
///   3) 尝试 overlay 全局热挂载 /system/bin、/system/lib —— 成功后新增命令/库立即生效；
///   4) overlay 不可用时不覆盖任何系统文件，新增项已持久化、重启后自动生效。
pub fn mount_lib_hot(dir: &str, id: &str) -> Result<(), String> {
    // 1) 持久复制（幂等、安全：系统已有同名一律跳过）
    mount_lib(dir, id)?;

    // 2) 安全兜底：清理会覆盖系统文件的旧残留
    prune_system_conflicts(dir, id);

    // 3) overlay 热挂载（即时生效）
    let bin_hot = overlay_hot_mount(
        dir,
        "/system/bin",
        &format!("{dir}/system/bin"),
        &format!("{dir}/overlay/work/bin"),
    );
    let lib_hot = overlay_hot_mount(
        dir,
        "/system/lib",
        &format!("{dir}/system/lib"),
        &format!("{dir}/overlay/work/lib"),
    );
    if bin_hot || lib_hot {
        // 即时生效：upper 即模块 system/，文件已在里面；顺手给新 bin 设系统上下文（尽力）
        let meta = crate::repo::get_meta(dir, id)?;
        for b in &meta.bins {
            let _ = util::run(
                &format!("chcon -h u:object_r:system_file:s0 '{dir}/system/bin/{b}' 2>/dev/null || true"),
                None,
            );
        }
        util::log_file(dir, &format!("热挂载即时生效: {id}"));
        println!("已挂载并即时生效: {id}");
        return Ok(());
    }

    // 4) overlay 不可用：只持久化，不覆盖任何系统文件，新增项重启后生效
    util::log_file(
        dir,
        &format!("热挂载不可用，{id} 已持久化（重启后自动生效；未覆盖任何系统文件）"),
    );
    println!("已挂载: {id}（已持久化到模块，重启后自动生效；未覆盖任何系统文件）");
    Ok(())
}
