//! LibPool 原生管理工具（Rust）
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
//! 功能：list / status / install / remove / mount / unmount / toggle / apply / ensure-core / reset
//! 设计要点：
//! - 仅依赖 std + 内置极简 JSON（零外部 crate，方便纯静态交叉编译）
//! - 所有下载 / 解压动作调用外部工具（curl / busybox wget / tar / xz / unzip），
//!   这些工具本身也是模块捆绑库，保证开机即有。
//! - 挂载用 bind mount，卸载用 umount，均幂等。

mod ar;
mod json;
mod mount;
mod repo;
mod util;

use std::env;
use std::process::ExitCode;

const VERSION: &str = "1.0.0";

fn usage() -> String {
    r#"用法: libman <命令> [参数]

命令:
  list                列出所有库及状态（JSON）
  status              简要状态（JSON）
  install <id>        下载并安装扩展库
  remove <id>         删除已安装的库（含卸载）
  mount <id>          挂载指定库到 /system/bin、/system/lib
  unmount <id>        卸载指定库
  toggle <id>         一键开关（缺库自动安装）
  apply               按已保存状态重放挂载（开机用）
  ensure-core         补装缺失的核心库（安装/开机用）
  reset               卸载全部并清理
  config <key> <val>  读取/设置配置（如 mirror）
  version             打印版本
"#
    .to_string()
}

fn main() -> ExitCode {
    // 模块根目录：优先环境变量 LIBPOOL_DIR，默认 /data/adb/modules/libpool
    let dir = env::var("LIBPOOL_DIR")
        .unwrap_or_else(|_| "/data/adb/modules/libpool".to_string());

    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprint!("{}", usage());
        return ExitCode::from(1);
    }
    util::log_file(&dir, &format!("执行命令: {}", args.join(" ")));

    let result = match args[0].as_str() {
        "list" => repo::cmd_list(&dir),
        "status" => repo::cmd_status(&dir),
        "install" => {
            if args.len() < 2 {
                eprintln!("缺少库 id");
                return ExitCode::from(1);
            }
            install_cmd(&dir, &args[1])
        }
        "remove" => {
            if args.len() < 2 {
                eprintln!("缺少库 id");
                return ExitCode::from(1);
            }
            remove_cmd(&dir, &args[1])
        }
        "mount" => {
            if args.len() < 2 {
                eprintln!("缺少库 id");
                return ExitCode::from(1);
            }
            mount_cmd(&dir, &args[1])
        }
        "unmount" => {
            if args.len() < 2 {
                eprintln!("缺少库 id");
                return ExitCode::from(1);
            }
            unmount_cmd(&dir, &args[1])
        }
        "toggle" => {
            if args.len() < 2 {
                eprintln!("缺少库 id");
                return ExitCode::from(1);
            }
            toggle_cmd(&dir, &args[1])
        }
        "apply" => apply_cmd(&dir),
        "ensure-core" => ensure_core_cmd(&dir),
        "reset" => reset_cmd(&dir),
        "config" => config_cmd(&dir, &args[1..]),
        "version" => {
            println!("libman {VERSION}");
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print!("{}", usage());
            Ok(())
        }
        other => {
            eprintln!("未知命令: {other}");
            eprint!("{}", usage());
            Err("unknown command".to_string())
        }
    };

    match result {
        Ok(()) => {
            util::log_file(&dir, &format!("命令完成: {}", args[0]));
            ExitCode::SUCCESS
        }
        Err(e) => {
            util::log_file(&dir, &format!("命令失败: {} -> {e}", args[0]));
            // 输出对 WebUI 友好的 JSON 错误（若以 --json 前缀则含 JSON 标记）
            eprintln!("libman 错误: {e}");
            ExitCode::from(1)
        }
    }
}

fn install_cmd(dir: &str, id: &str) -> Result<(), String> {
    let repos = repo::load_repos(dir)?;
    let entry = repos
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| format!("仓库中不存在库: {id}"))?;
    repo::install_lib(dir, entry, true)
}

fn remove_cmd(dir: &str, id: &str) -> Result<(), String> {
    // 先卸载（若有挂载），再删除文件与状态
    let _ = mount::unmount_lib_by_id(dir, id);
    let _ = repo::mark_mounted(dir, id, false);
    let lib_root = format!("{dir}/libs/{id}");
    let _ = util::run(&format!("rm -rf {lib_root}"), None);
    println!("已删除库: {id}");
    Ok(())
}

fn mount_cmd(dir: &str, id: &str) -> Result<(), String> {
    let installed = repo::is_installed(dir, id);
    if !installed {
        return Err(format!("库未安装: {id}，请先下载"));
    }
    mount::mount_lib(dir, id)?;
    repo::mark_mounted(dir, id, true)
}

fn unmount_cmd(dir: &str, id: &str) -> Result<(), String> {
    mount::unmount_lib_by_id(dir, id)?;
    repo::mark_mounted(dir, id, false)
}

fn toggle_cmd(dir: &str, id: &str) -> Result<(), String> {
    let mounted = repo::is_mounted(dir, id)?;
    if mounted {
        unmount_cmd(dir, id)
    } else {
        // 未安装则先装
        if !repo::is_installed(dir, id) {
            install_cmd(dir, id)?;
        }
        mount_cmd(dir, id)
    }
}

fn apply_cmd(dir: &str) -> Result<(), String> {
    let mounted_list = repo::get_mounted_ids(dir)?;
    let mut ok = 0usize;
    let mut fail = 0usize;
    for id in &mounted_list {
        if !repo::is_installed(dir, id) {
            eprintln!("跳过（未安装）: {id}");
            continue;
        }
        match mount::mount_lib(dir, id) {
            Ok(()) => ok += 1,
            Err(e) => {
                fail += 1;
                eprintln!("挂载失败 {id}: {e}");
            }
        }
    }
    println!("apply 完成: 成功 {ok} 个, 失败 {fail} 个");
    Ok(())
}

fn ensure_core_cmd(dir: &str) -> Result<(), String> {
    let repos = repo::load_repos(dir)?;
    let mut done = 0usize;
    for entry in repos.iter().filter(|r| r.core) {
        if repo::is_installed(dir, &entry.id) {
            continue;
        }
        eprintln!("自动安装核心库: {}", entry.id);
        repo::install_lib(dir, entry, false)?;
        done += 1;
    }
    println!("核心库就绪检查完成，新装 {done} 个");
    Ok(())
}

fn reset_cmd(dir: &str) -> Result<(), String> {
    mount::unmount_all(dir)?;
    repo::reset_state(dir)?;
    println!("已全部卸载并清空状态");
    Ok(())
}

fn config_cmd(dir: &str, args: &[String]) -> Result<(), String> {
    match args {
        [] => {
            let cfg = util::load_config(dir)?;
            println!("{}", json::to_string_pretty(&json::Json::Obj({
                let mut m = std::collections::BTreeMap::new();
                m.insert("mirror".to_string(), json::Json::Str(cfg.mirror));
                m.insert("arch".to_string(), json::Json::Str(cfg.arch));
                m
            })));
            Ok(())
        }
        [key, value] => {
            let mut cfg = util::load_config(dir)?;
            cfg.mirror = if key == "mirror" {
                value.clone()
            } else {
                cfg.mirror
            };
            util::save_config(dir, &cfg)?;
            println!("已保存 {key} = {value}");
            Ok(())
        }
        _ => Err("config 用法: libman config [mirror <url>]".to_string()),
    }
}
