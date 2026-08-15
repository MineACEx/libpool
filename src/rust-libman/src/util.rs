//! 工具函数：执行命令、下载、配置读写、xz 解压等
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
use std::io::Read;
use std::process::{Command, Stdio};

use crate::json::{self, Json};

pub struct Config {
    pub mirror: String,
    pub arch: String,
}

fn default_mirror() -> String {
    "https://packages.termux.dev/apt/termux-main".to_string()
}
fn default_arch() -> String {
    "aarch64".to_string()
}

pub fn load_config(dir: &str) -> Result<Config, String> {
    let p = format!("{dir}/libs/config.json");
    match std::fs::read_to_string(&p) {
        Ok(s) => {
            let j = json::parse(&s).map_err(|e| format!("配置文件损坏: {e}"))?;
            j.as_obj().map_err(|_| "配置不是对象".to_string())?;
            Ok(Config {
                mirror: j.get_str("mirror").unwrap_or_else(default_mirror),
                arch: j.get_str("arch").unwrap_or_else(default_arch),
            })
        }
        Err(_) => Ok(Config {
            mirror: default_mirror(),
            arch: default_arch(),
        }),
    }
}

pub fn save_config(dir: &str, cfg: &Config) -> Result<(), String> {
    let p = format!("{dir}/libs/config.json");
    let j = Json::Obj({
        let mut m = std::collections::BTreeMap::new();
        m.insert("mirror".to_string(), Json::Str(cfg.mirror.clone()));
        m.insert("arch".to_string(), Json::Str(cfg.arch.clone()));
        m
    });
    let s = json::to_string_pretty(&j);
    std::fs::write(&p, s).map_err(|e| format!("写入配置失败: {e}"))
}

/// 执行命令，返回 (status, stdout, stderr)
pub fn run(cmdline: &str, timeout_secs: Option<u64>) -> Result<(bool, String, String), String> {
    let (shell, prefix) = find_shell();
    let mut cmd = Command::new(&shell);
    for a in &prefix {
        cmd.arg(a);
    }
    cmd.arg("-c")
        .arg(cmdline)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let path = std::env::var("PATH").unwrap_or_default();
    let base = if path.is_empty() {
        "/system/bin:/system/xbin:/sbin:/data/adb/ksu/bin:/data/adb/magisk:/data/adb/modules/libpool/libs/core/bin:/data/adb/modules/libpool/tools".to_string()
    } else {
        format!("{path}:/system/bin:/system/xbin:/sbin:/data/adb/ksu/bin:/data/adb/magisk:/data/adb/modules/libpool/libs/core/bin:/data/adb/modules/libpool/tools")
    };
    cmd.env("PATH", &base);
    cmd.env("LD_LIBRARY_PATH", "/data/adb/modules/libpool/libs/core/lib:/system/lib64:/system/lib");

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动命令 {cmdline}: {e}"))?;
    let mut out = String::new();
    let mut err = String::new();
    let deadline = timeout_secs.map(|s| std::time::Instant::now() + std::time::Duration::from_secs(s));
    let status = loop {
        if let Some(d) = deadline {
            if std::time::Instant::now() > d {
                let _ = child.kill();
                return Err(format!("命令超时: {cmdline}"));
            }
        }
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let _ = child.stdout.take().map(|mut o| o.read_to_string(&mut out));
    let _ = child.stderr.take().map(|mut o| o.read_to_string(&mut err));
    Ok((status.success(), out, err))
}

/// 找到可用的 shell。返回 (可执行文件路径, 前缀参数)。
fn find_shell() -> (String, Vec<String>) {
    let busybox_candidates = [
        "/data/adb/ksu/bin/busybox",
        "/data/adb/magisk/busybox",
    ];
    for c in busybox_candidates {
        if std::path::Path::new(c).exists() {
            return (
                c.to_string(),
                vec!["sh".to_string(), "-o".to_string(), "standalone".to_string()],
            );
        }
    }
    for c in ["/system/bin/sh", "/system/bin/mksh"] {
        if std::path::Path::new(c).exists() {
            return (c.to_string(), Vec::new());
        }
    }
    ("sh".to_string(), Vec::new())
}

/// 下载文件到指定路径。
pub fn download(url: &str, dest: &str) -> Result<(), String> {
    let _ = std::fs::create_dir_all(std::path::Path::new(dest).parent().unwrap_or(std::path::Path::new("/")));
    let cmds = [
        format!("curl -fsSL --connect-timeout 15 --retry 2 -o {dest} '{url}'"),
        format!("busybox wget -q -T 20 -O {dest} '{url}'"),
        format!("wget -q -T 20 -O {dest} '{url}'"),
    ];
    for c in &cmds {
        if let Ok((ok, _, _)) = run(c, Some(120)) {
            if ok && std::path::Path::new(dest).exists() {
                let size = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
                if size > 0 {
                    return Ok(());
                }
            }
        }
    }
    Err(format!("下载失败: {url}"))
}

/// xz 解压
pub fn xz_decompress(src: &str, out: &str) -> Result<(), String> {
    let cmds = [
        format!("xz -dc {src} > {out}"),
        format!("busybox unxz -c {src} > {out}"),
        format!("unxz -c {src} > {out}"),
    ];
    for c in &cmds {
        if let Ok((ok, _, _)) = run(c, Some(120)) {
            if ok {
                return Ok(());
            }
        }
    }
    Err("xz 解压失败（缺少 xz 工具）".to_string())
}