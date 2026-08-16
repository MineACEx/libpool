//! Magisk / root 检测隐藏：用 bind mount 把常见检测路径覆盖为安全的空文件。
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
//!
//! 原理：
//!   检测软件常通过检查 /system/bin/su、/system/xbin/su、/sbin/su、/sbin/.magisk
//!   等路径是否存在来判断是否 root / Magisk。libman hide 把这些路径用 bind mount
//!   覆盖成一个空的普通文件（文件在，但不是 su），让浅层检测误判"未安装"。
//!
//! 特性：
//!   - **一次性命令**：执行完立即退出，无驻留进程、零功耗（满足"几乎无功耗"）。
//!   - 可随时 restore 还原；状态记录在 hide/state.json。
//!   - 只 bind 实际存在的路径，不新增/删除系统文件，可完全撤销、不破坏系统。
//!
//! 局限（诚实说明）：
//!   - 无法对抗读取 /proc/self/mountinfo、遍历 /proc 的深度检测（这类需内核级隐藏）。
//!   - 各厂商自有验机逻辑（如小米新机检测）不在覆盖范围。

use crate::json::{self, Json};
use crate::util;
use std::path::Path;
use std::fs;

/// 常见 Magisk / root 检测路径（先探测存在性，只处理系统里真实存在的）
const DETECT_PATHS: &[&str] = &[
    "/system/bin/su",
    "/system/xbin/su",
    "/sbin/su",
    "/debug_ramdisk/su",
    "/system/bin/magisk",
    "/system/bin/magiskinit",
    "/sbin/magisk",
    "/sbin/magiskinit",
    "/sbin/.magisk",
];

fn state_path(dir: &str) -> String {
    format!("{dir}/hide/state.json")
}
fn stub_dir(dir: &str) -> String {
    format!("{dir}/hide/stub")
}

fn write_state(dir: &str, applied: &[String]) {
    let s = format!(
        "{{ \"applied\": [{}] }}\n",
        applied
            .iter()
            .map(|p| format!(
                "\"{}\"",
                p.replace('\\', "\\\\").replace('"', "\\\"")
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = std::fs::create_dir_all(Path::new(&state_path(dir)).parent().unwrap());
    let _ = std::fs::write(state_path(dir), s);
}

fn read_state(dir: &str) -> Vec<String> {
    let s = std::fs::read_to_string(state_path(dir)).unwrap_or_default();
    let mut out = Vec::new();
    for tok in s.split('"').skip(1).step_by(2) {
        if tok.starts_with('/') {
            out.push(tok.to_string());
        }
    }
    out
}

/// 是否已应用（有任何路径被覆盖）
pub fn is_applied(dir: &str) -> bool {
    !read_state(dir).is_empty()
}

/// 执行隐藏：bind mount 覆盖存在的检测路径（幂等，重复执行只补缺）
pub fn hide_apply(dir: &str) -> Result<(), String> {
    let stub = stub_dir(dir);
    std::fs::create_dir_all(&stub).map_err(|e| e.to_string())?;
    let already = read_state(dir);
    let mut applied: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for p in DETECT_PATHS {
        if !Path::new(p).exists() {
            continue; // 系统里本就没有，无需处理
        }
        if already.iter().any(|a| a == p) {
            continue; // 已由我们覆盖
        }
        let base = p.rsplit('/').next().unwrap_or("stub");
        let stub_path = format!("{stub}/{base}");
        let _ = std::fs::write(&stub_path, b"");
        match util::run(&format!("mount --bind '{stub_path}' '{p}'"), Some(10)) {
            Ok(_) => {
                applied.push(p.to_string());
                println!("已隐藏: {p}");
            }
            Err(e) => skipped.push(format!("{p} ({e})")),
        }
    }
    let mut all = already;
    all.extend(applied.iter().cloned());
    write_state(dir, &all);
    util::log_file(
        dir,
        &format!("hide apply: 本次覆盖 {} 个，跳过 {} 个", applied.len(), skipped.len()),
    );
    for s in &skipped {
        util::log_file(dir, &format!("hide 跳过: {s}"));
    }
    if applied.is_empty() && all.is_empty() {
        Err("没有可覆盖的路径（可能设备无这些检测点或不支持 bind mount）".to_string())
    } else {
        Ok(())
    }
}

/// 还原：umount 我们覆盖过的路径，删除 stub
pub fn hide_restore(dir: &str) -> Result<(), String> {
    let applied = read_state(dir);
    let mut ok = 0;
    for p in &applied {
        if util::run(&format!("umount '{p}'"), Some(10)).is_ok() {
            ok += 1;
        }
    }
    let _ = std::fs::remove_dir_all(stub_dir(dir));
    write_state(dir, &[]);
    util::log_file(dir, &format!("hide restore: 还原 {} 个路径", ok));
    println!("已还原 {ok} 个隐藏路径");
    Ok(())
}

/// 查看当前覆盖状态
pub fn hide_status(dir: &str) -> Result<(), String> {
    let applied = read_state(dir);
    println!("{{ \"applied\": {} }}", applied.len());
    for p in &applied {
        println!("  {p}");
    }
    Ok(())
}

/* ============================================================================
   按应用深度隐藏（hide apps …）
   ----------------------------------------------------------------------------
   适用：MOMO/淘宝/银行等针对"隐藏 Magisk/KernelSU 环境"的应用。
   原理：把目标应用的包名写进 KernelSU 的 denylist/umount 列表，让该应用在
   运行时看不到本模块的挂载、su 等 root 痕迹（KernelSU 会为该应用彻底 umount
   模块挂载 + 屏蔽 su 暴露）。Magisk 侧对应 denyList（.db）。
   设计：
     - 配置文件独立于全局 hide：hide/apps.json，仅管"包名 + 是否启用"。
     - 启用/关闭时立即尝试写入 Linux 侧的 denylist（经 ksud / 直接写 denylist），
       失败也用真实原因写日志、不误报成功（本地记录仍保留，供开机重放）。
     - 开机重放由 libman 内部循环 hide/apps.json 里"已启用"的包名调用 set(on)。
   ========================================================================= */

fn apps_path(dir: &str) -> String {
    format!("{dir}/hide/apps.json")
}

/// 读取按应用深度隐藏配置 → Vec<(包名, 是否启用)>
fn apps_load(dir: &str) -> Vec<(String, bool)> {
    let s = fs::read_to_string(apps_path(dir)).unwrap_or_default();
    let mut out: Vec<(String, bool)> = Vec::new();
    if let Ok(Json::Obj(m)) = json::parse(&s) {
        if let Some(Json::Arr(arr)) = m.get("apps") {
            for item in arr {
                if matches!(item, Json::Obj(_)) {
                    let pkg = item.get_str("pkg").unwrap_or_default();
                    let on = item.get_bool("enabled").unwrap_or(false);
                    if !pkg.is_empty() {
                        out.push((pkg, on));
                    }
                }
            }
        }
    }
    out
}

/// 保存按应用深度隐藏配置
fn apps_save(dir: &str, list: &[(String, bool)]) -> Result<(), String> {
    let dir_path = Path::new(&apps_path(dir)).parent().unwrap().to_path_buf();
    fs::create_dir_all(&dir_path).map_err(|e| e.to_string())?;
    let mut s = String::from("{\n  \"apps\": [\n");
    for (i, (pkg, on)) in list.iter().enumerate() {
        let comma = if i + 1 == list.len() { "" } else { "," };
        s.push_str(&format!("    {{ \"pkg\": \"{pkg}\", \"enabled\": {} }}{comma}\n", if *on { "true" } else { "false" }));
    }
    s.push_str("  ]\n}\n");
    fs::write(apps_path(dir), s).map_err(|e| e.to_string())
}

/// KernelSU 侧的命令位置候选（denylist 操作）
const KSUD_CANDIDATES: &[&str] = &[
    "/data/adb/ksu/bin/ksud",
    "/data/adb/ksu/bin/ksu",
    "/data/adb/modules/zygisk_next/bin/ksud",
];

/// KernelSU denylist 文件（无 ksud 二进制时的直写回退）
const KSU_DENYLIST: &str = "/data/adb/ksu/denylist";

/// 对某个包启用/关闭 KernelSU denylist（深度隐藏）。返回执行说明。
fn apply_denylist(pkg: &str, on: bool) -> String {
    let verb = if on { "add" } else { "rm" };
    // 1) 优先用 ksud 命令（若存在）
    for k in KSUD_CANDIDATES {
        if Path::new(k).exists() {
            match util::run(&format!("{k} denylist {verb} {pkg}"), Some(10)) {
                Ok((true, _, _)) => return format!("已通过 {k} 把 {pkg} {verb} 进 denylist"),
                Ok((false, _, err)) => return format!("{k} 执行失败: {}", err.trim_end()),
                Err(e) => return format!("{k} 调用失败: {e}"),
            }
        }
    }
    // 2) 回退：直写 denylist 文件（每行含此包名；启用时追加、关闭时剔除）
    let cur = fs::read_to_string(KSU_DENYLIST).unwrap_or_default();
    let mut lines: Vec<String> = cur
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if on {
        if !lines.iter().any(|l| l.ends_with(pkg)) {
            lines.push(pkg.to_string());
        }
    } else {
        lines.retain(|l| !l.ends_with(pkg));
    }
    match fs::write(KSU_DENYLIST, lines.join("\n") + "\n") {
        Ok(()) => format!("已直写 {KSU_DENYLIST}：{pkg} {}", if on { "启用" } else { "关闭" }),
        Err(e) => format!("写入 {KSU_DENYLIST} 失败: {e}"),
    }
}

/// 扫描已安装的三方应用包名（供页面下拉选择），返回 JSON 数组。
/// 用 `pm list packages -3`（第三方应用），输出被过滤为纯包名。
pub fn hide_apps_scan(dir: &str) -> Result<(), String> {
    let (ok, out, err) = util::run("pm list packages -3 2>/dev/null | sed 's/^package://'", Some(20))?;
    if !ok || out.trim().is_empty() {
        return Err(format!("无法列出已装应用（pm 不可用或为空）：{}", err.trim()));
    }
    let mut pkgs: Vec<String> = out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    pkgs.sort();
    pkgs.dedup();
    println!("[{}]", pkgs.iter().map(|p| format!("\"{p}\"")).collect::<Vec<_>>().join(", "));
    util::log_file(dir, &format!("hide apps scan: {} 个三方应用", pkgs.len()));
    Ok(())
}

/// 列出当前深度隐藏配置，输出 JSON（供 WebUI）：[{"pkg":..., "enabled":bool}]
pub fn hide_apps_list(dir: &str) -> Result<(), String> {
    let list = apps_load(dir);
    let items = list
        .iter()
        .map(|(pkg, on)| format!("{{\"pkg\":\"{pkg}\",\"enabled\":{}}}", if *on { "true" } else { "false" }))
        .collect::<Vec<_>>()
        .join(",");
    println!("[{}]", items);
    Ok(())
}

/// 启用/关闭某应用的深度隐藏。立即尝试写入 denylist，并持久化到 apps.json。
pub fn hide_apps_set(dir: &str, pkg: &str, on: bool) -> Result<(), String> {
    if pkg.is_empty() {
        return Err("缺少包名".to_string());
    }
    let list = apps_load(dir);
    let mut found = false;
    let mut new_list: Vec<(String, bool)> = Vec::new();
    for (p, o) in &list {
        if p == pkg {
            new_list.push((pkg.to_string(), on));
            found = true;
        } else {
            new_list.push((p.clone(), *o));
        }
    }
    if !found {
        new_list.push((pkg.to_string(), on));
    }
    apps_save(dir, &new_list)?;
    let note = apply_denylist(pkg, on);
    util::log_file(
        dir,
        &format!(
            "hide apps set {} {}：{}",
            pkg,
            if on { "on" } else { "off" },
            note
        ),
    );
    println!(
        "{} 深度隐藏已{}。{}",
        pkg,
        if on { "启用" } else { "关闭" },
        note
    );
    Ok(())
}

/// 按应用深度隐藏状态概要
pub fn hide_apps_status(dir: &str) -> Result<(), String> {
    let list = apps_load(dir);
    let enabled = list.iter().filter(|(_, on)| *on).count();
    println!("{{ \"apps\": {}, \"enabled\": {} }}", list.len(), enabled);
    Ok(())
}

/// `hide apps …` 子命令分发
pub fn hide_apps_cmd(dir: &str, args: &[String]) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        Some("scan") => hide_apps_scan(dir),
        Some("list") => hide_apps_list(dir),
        Some("status") => hide_apps_status(dir),
        Some("sel") | Some("set") => {
            if args.len() < 3 {
                return Err("用法: libman hide apps set <pkg> <on|off>".to_string());
            }
            hide_apps_set(dir, &args[1], args[2] == "on" || args[2] == "true")
        }
        Some(other) => Err(format!(
            "未知 hide apps 子命令: {other}（可用: scan / list / set <pkg> <on|off>）"
        )),
        None => Err("hide apps 需要子命令: scan / list / set <pkg> <on|off>".to_string()),
    }
}

/// 开机/执行 apply 时重放：把 apps.json 里「已启用」的包重新写进 KernelSU denylist。
/// 幂等（denylist add 已存在包无副作用）；设备重启后保证深度隐藏配置不丢失。
pub fn hide_apps_replay(dir: &str) -> Result<(), String> {
    let list = apps_load(dir);
    let enabled: Vec<String> = list
        .iter()
        .filter(|(_, on)| *on)
        .map(|(p, _)| p.clone())
        .collect();
    if enabled.is_empty() {
        return Ok(());
    }
    let mut ok = 0usize;
    for p in &enabled {
        let note = apply_denylist(p, true);
        if note.contains("已通过") || note.contains("已直写") {
            ok += 1;
        }
        util::log_file(dir, &format!("hide apps 重放 {p}: {note}"));
    }
    println!("深度隐藏重放: {ok}/{} 个已启用包已写回 denylist", enabled.len());
    Ok(())
}
