//! 工具函数：执行命令、下载、配置读写、xz 解压、文件日志等
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
use std::io::Read;
use std::process::{Command, Stdio};

use crate::json::{self, Json};

/// 把一行追加到模块日志 log/libman.log（best-effort，写失败不影响功能）。
/// 与 libman.sh 共用同一个日志文件，WebUI「设置 → 查看日志」可读。
pub fn log_file(dir: &str, msg: &str) {
    let path = format!("{dir}/log/libman.log");
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "[{}] libman: {}", utc_now_str(), msg);
    }
}

/// UTC 时间戳字符串（YYYY-MM-DD HH:MM:SS），纯手工换算，零依赖。
fn utc_now_str() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // 天数 -> 公历（Howard Hinnant 算法）
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, hh, mm, ss)
}

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
///
/// 顺序：
/// 1. shell 工具（curl / busybox wget / wget）——设备自带可用 TLS 时最稳，支持 https；
/// 2. 纯 Rust 的 http_get（仅 http://，零 TLS 依赖）——设备缺 curl/wget 且镜像走明文 HTTP 时兜底。
pub fn download(url: &str, dest: &str) -> Result<(), String> {
    let _ = std::fs::create_dir_all(std::path::Path::new(dest).parent().unwrap_or(std::path::Path::new("/")));
    let cmds = [
        format!("curl -fsSL --connect-timeout 15 --retry 2 -o {dest} '{url}'"),
        format!("busybox wget -q -T 20 -O {dest} '{url}'"),
        format!("wget -q -T 20 -O {dest} '{url}'"),
    ];
    for c in &cmds {
        if let Ok((ok, _, _)) = run(c, Some(120)) {
            if ok && file_nonempty(dest) {
                return Ok(());
            }
        }
    }
    // 纯 Rust HTTP：只支持 http://；https:// 必须依赖 shell 的 TLS 工具。
    if url.starts_with("http://") {
        let _ = std::fs::remove_file(dest);
        if http_get(url, dest, 120).is_ok() && file_nonempty(dest) {
            return Ok(());
        }
    }
    let _ = std::fs::remove_file(dest);
    let _ = std::fs::remove_file(&format!("{dest}.progress"));
    Err(format!("下载失败: {url}"))
}

fn file_nonempty(p: &str) -> bool {
    std::fs::metadata(p).map(|m| m.len() > 0).unwrap_or(false)
}

/// 找到响应头结束位置（\r\n\r\n 的末尾），返回头部总长度；找不到返回 None。
fn find_crlfcrlf(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

/// 极简 HTTP/1.1 GET 客户端（仅 http://，无 TLS 依赖）。
///
/// 用于设备缺少 curl/wget（或 busybox wget 不支持 TLS）时下载明文 HTTP 镜像。
/// 支持 Content-Length 与 chunked 响应体；支持 http->http 重定向（最多 6 跳，
/// http->https 不支持，会明确报错提示改用支持明文 HTTP 的镜像）。
/// 成功则把响应体写入 dest；任何一步失败返回 Err。
pub fn http_get(url: &str, dest: &str, timeout_secs: u64) -> Result<(), String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut cur_url = url.to_string();
    for _hop in 0..6 {
        // ---- 解析 http:// URL ----
        let rest = cur_url
            .strip_prefix("http://")
            .ok_or_else(|| format!("http_get 仅支持 http://: {cur_url}"))?;
        let (hostport, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (host, port) = match hostport.find(':') {
            Some(i) => (&hostport[..i], hostport[i + 1..].parse::<u16>().unwrap_or(80)),
            None => (hostport, 80u16),
        };
        if host.is_empty() {
            return Err(format!("URL 缺少主机名: {cur_url}"));
        }

        let mut stream = TcpStream::connect((host, port))
            .map_err(|e| format!("连接 {host}:{port} 失败: {e}"))?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(timeout_secs)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(timeout_secs)));

        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: libman/1.0\r\nAccept: */*\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(req.as_bytes())
            .map_err(|e| format!("发送请求失败: {e}"))?;

        // ---- 读响应头（把可能多读的 body 首字节一并保留在 overflow）----
        let mut raw = Vec::new();
        let mut buf = [0u8; 4096];
        let mut head_end = None;
        while head_end.is_none() {
            let n = stream
                .read(&mut buf)
                .map_err(|e| format!("读取响应头失败: {e}"))?;
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&buf[..n]);
            head_end = find_crlfcrlf(&raw);
            if raw.len() > 1 << 16 {
                return Err("响应头过大".to_string());
            }
        }
        let head_end = head_end.ok_or("响应头不完整")?;
        let head_str = String::from_utf8_lossy(&raw[..head_end]).to_string();

        let mut code = 0u16;
        let mut location = String::new();
        let mut chunked = false;
        let mut content_length: Option<u64> = None;
        for (i, line) in head_str.lines().enumerate() {
            let l = line.trim();
            if i == 0 {
                if let Some(c) = l.split_whitespace().nth(1) {
                    code = c.parse().unwrap_or(0);
                }
            } else if let Some(v) = l.strip_prefix("Location:") {
                location = v.trim().to_string();
            } else if l.eq_ignore_ascii_case("Transfer-Encoding: chunked") {
                chunked = true;
            } else if let Some(v) = l.strip_prefix("Content-Length:") {
                content_length = v.trim().parse().ok();
            }
        }
        let overflow = raw[head_end..].to_vec();

        // ---- 重定向处理 ----
        if (300..400).contains(&code) {
            if location.starts_with("http://") {
                cur_url = location;
                continue;
            }
            return Err(format!("服务端 {code} 重定向到不支持的目标: {location}"));
        }
        if code != 200 {
            return Err(format!("HTTP 状态 {code}"));
        }

        // ---- 写出响应体 ----
        let mut out = std::fs::File::create(dest).map_err(|e| e.to_string())?;
        let mut total: u64 = 0; // 已写出字节数
        let mut cursor = 0usize; // overflow 内已消费的字节数
        // 进度文件（供 WebUI 显示真实下载百分比）：内容 "done total"，下载完删除
        let progress_path = format!("{dest}.progress");
        let mut last_pct: u32 = 0;
        if let Some(len) = content_length {
            let _ = std::fs::write(&progress_path, format!("0 {len}"));
        }
        // 逐字节读取器：先消费 overflow，再读网络流
        let read_byte = |s: &mut TcpStream,
                         over: &[u8],
                         i: &mut usize|
         -> Result<Option<u8>, String> {
            if *i < over.len() {
                let b = over[*i];
                *i += 1;
                Ok(Some(b))
            } else {
                let mut b = [0u8; 1];
                let n = s.read(&mut b).map_err(|e| format!("读取响应体失败: {e}"))?;
                if n == 0 {
                    Ok(None)
                } else {
                    Ok(Some(b[0]))
                }
            }
        };

        if chunked {
            loop {
                // 读取 chunk 大小行（到 \r\n 为止）
                let mut sizeline = Vec::new();
                loop {
                    match read_byte(&mut stream, &overflow, &mut cursor)? {
                        Some(b) => {
                            sizeline.push(b);
                            if sizeline.len() >= 2 && sizeline.ends_with(b"\r\n") {
                                break;
                            }
                            if sizeline.len() > 64 {
                                return Err("chunk 大小行异常".to_string());
                            }
                        }
                        None => return Err("chunk 读取中断".to_string()),
                    }
                }
                let line = String::from_utf8_lossy(&sizeline[..sizeline.len() - 2]).to_string();
                let hexpart = line.split(';').next().unwrap_or("").trim();
                let size = usize::from_str_radix(hexpart, 16)
                    .map_err(|_| format!("chunk 大小解析失败: {line}"))?;
                if size == 0 {
                    let _ = read_byte(&mut stream, &overflow, &mut cursor);
                    let _ = read_byte(&mut stream, &overflow, &mut cursor);
                    break;
                }
                for _ in 0..size {
                    match read_byte(&mut stream, &overflow, &mut cursor)? {
                        Some(b) => {
                            out.write_all(&[b]).map_err(|e| e.to_string())?;
                        }
                        None => return Err("chunk 数据中断".to_string()),
                    }
                }
                let _ = read_byte(&mut stream, &overflow, &mut cursor);
                let _ = read_byte(&mut stream, &overflow, &mut cursor);
            }
        } else if let Some(len) = content_length {
            // 先写出 overflow 里已有的 body
            let have = overflow.len() - cursor;
            let want = ((len - total) as usize).min(have);
            if want > 0 {
                out.write_all(&overflow[cursor..cursor + want])
                    .map_err(|e| e.to_string())?;
                total += want as u64;
            }
            let mut buf = [0u8; 65536];
            while total < len {
                let n = stream
                    .read(&mut buf)
                    .map_err(|e| format!("读取响应体失败: {e}"))?;
                if n == 0 {
                    break;
                }
                let take = ((len - total) as usize).min(n);
                out.write_all(&buf[..take]).map_err(|e| e.to_string())?;
                total += take as u64;
                // 真实进度上报（每 ≥2% 变化写一次）
                let pct = if len > 0 { ((total as f64 / len as f64) * 100.0) as u32 } else { 0 };
                if pct >= last_pct + 2 || pct == 100 {
                    let _ = std::fs::write(&progress_path, format!("{} {}", total, len));
                    last_pct = pct;
                }
                if take < n {
                    break;
                }
            }
        } else {
            // 无 Content-Length：读至连接关闭
            if cursor < overflow.len() {
                out.write_all(&overflow[cursor..]).map_err(|e| e.to_string())?;
                total += (overflow.len() - cursor) as u64;
            }
            let mut buf = [0u8; 65536];
            loop {
                let n = stream
                    .read(&mut buf)
                    .map_err(|e| format!("读取响应体失败: {e}"))?;
                if n == 0 {
                    break;
                }
                out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                total += n as u64;
            }
        }
        drop(out);
        if total == 0 {
            let _ = std::fs::remove_file(dest);
            let _ = std::fs::remove_file(&progress_path);
            return Err("下载内容为空".to_string());
        }
        let _ = std::fs::remove_file(&progress_path);
        return Ok(());
    }
    Err("重定向次数过多".to_string())
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