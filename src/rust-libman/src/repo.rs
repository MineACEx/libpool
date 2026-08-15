//! 仓库（repos.json）模型 + 安装/删除/状态逻辑
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::ar;
use crate::json::{self, Json};
use crate::util;

/// 扩展库信息（从 repos.json 解析）
pub struct RepoEntry {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub category: String,
    /// termux = 从 Termux 仓库安装；bundled = 核心库
    pub r#type: String,
    /// termux 包名
    pub pkg: String,
    /// 下载地址（type=url 用，当前未使用）
    pub url: String,
    /// 归档类型
    pub archive: String,
    /// 需要安装到 bin 的文件名
    pub bins: Vec<String>,
    /// 需要安装到 lib 的 .so 文件名
    pub libs: Vec<String>,
    /// 是否核心库
    pub core: bool,
    /// 版本号
    pub version: String,
}

/// 已安装库的元信息（每库一个，放在 libs/<id>/meta.json）
pub struct InstalledMeta {
    pub version: String,
    pub bins: Vec<String>,
    pub libs: Vec<String>,
    pub source_type: String,
}

impl RepoEntry {
    fn from_json(v: &Json) -> Option<RepoEntry> {
        Some(RepoEntry {
            id: v.get_str("id")?,
            name: v.get_str("name")?,
            desc: v.get_str("desc").unwrap_or_default(),
            category: v.get_str("category").unwrap_or_default(),
            r#type: v.get_str("type").unwrap_or_else(|| "termux".to_string()),
            pkg: v.get_str("pkg").unwrap_or_default(),
            url: v.get_str("url").unwrap_or_default(),
            archive: v.get_str("archive").unwrap_or_default(),
            bins: v.get_str_arr("bins"),
            libs: v.get_str_arr("libs"),
            core: v.get_bool("core").unwrap_or(false),
            version: v.get_str("version").unwrap_or_default(),
        })
    }
}

fn state_path(dir: &str) -> String {
    format!("{dir}/webroot/data/state.json")
}
fn repos_path(dir: &str) -> String {
    format!("{dir}/webroot/data/repos.json")
}

pub fn load_repos(dir: &str) -> Result<Vec<RepoEntry>, String> {
    let p = repos_path(dir);
    let s = fs::read_to_string(&p).map_err(|e| format!("读取仓库列表失败: {e}"))?;
    let json = json::parse(&s).map_err(|e| format!("仓库列表解析失败: {e}"))?;
    let arr = json.as_arr().map_err(|_| "仓库列表不是数组".to_string())?;
    let mut repos = Vec::new();
    for item in arr {
        if let Some(entry) = RepoEntry::from_json(item) {
            repos.push(entry);
        }
    }
    Ok(repos)
}

pub fn load_state(dir: &str) -> Result<Json, String> {
    let p = state_path(dir);
    match fs::read_to_string(&p) {
        Ok(s) => json::parse(&s).map_err(|e| format!("状态文件损坏: {e}")),
        Err(_) => Ok(Json::obj()),
    }
}

pub fn save_state(dir: &str, state: &Json) -> Result<(), String> {
    let p = state_path(dir);
    if let Some(parent) = Path::new(&p).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let s = json::to_string_pretty(state);
    fs::write(&p, s).map_err(|e| format!("写入状态失败: {e}"))
}

pub fn is_installed(dir: &str, id: &str) -> bool {
    Path::new(&format!("{dir}/libs/{id}/meta.json")).exists()
}

pub fn is_mounted(dir: &str, id: &str) -> Result<bool, String> {
    let st = load_state(dir)?;
    let libs = st.get("libs").and_then(|v| match v {
        Json::Obj(m) => Some(m),
        _ => None,
    });
    match libs.and_then(|m| m.get(id)) {
        Some(Json::Obj(o)) => Ok(o.get("mounted").and_then(|v| match v {
            Json::Bool(b) => Some(*b),
            _ => None,
        }).unwrap_or(false)),
        _ => Ok(false),
    }
}

pub fn mark_mounted(dir: &str, id: &str, mounted: bool) -> Result<(), String> {
    let mut st = load_state(dir)?;
    if let Json::Obj(ref mut root) = &mut st {
        let libs_obj = root.entry("libs".to_string()).or_insert_with(|| Json::obj());
        if let Json::Obj(ref mut m) = libs_obj {
            let mut entry_map = BTreeMap::new();
            entry_map.insert("mounted".to_string(), Json::Bool(mounted));
            m.insert(id.to_string(), Json::Obj(entry_map));
        }
    }
    save_state(dir, &st)
}

pub fn get_mounted_ids(dir: &str) -> Result<Vec<String>, String> {
    let st = load_state(dir)?;
    let mut ids = Vec::new();
    if let Some(Json::Obj(m)) = st.get("libs") {
        for (k, v) in m {
            let mounted = match v {
                Json::Obj(ref o) => o.get("mounted").and_then(|x| match x {
                    Json::Bool(b) => Some(*b),
                    _ => None,
                }).unwrap_or(false),
                _ => false,
            };
            if mounted {
                ids.push(k.clone());
            }
        }
    }
    Ok(ids)
}

pub fn reset_state(dir: &str) -> Result<(), String> {
    save_state(dir, &Json::obj())
}

pub fn get_meta(dir: &str, id: &str) -> Result<InstalledMeta, String> {
    let p = format!("{dir}/libs/{id}/meta.json");
    let s = fs::read_to_string(&p).map_err(|e| format!("读取 {id} 元信息失败: {e}"))?;
    let j = json::parse(&s).map_err(|e| format!("元信息损坏: {e}"))?;
    Ok(InstalledMeta {
        version: j.get_str("version").unwrap_or_default(),
        bins: j.get_str_arr("bins"),
        libs: j.get_str_arr("libs"),
        source_type: j.get_str("source_type").unwrap_or_default(),
    })
}

/// 安装库
pub fn install_lib(dir: &str, entry: &RepoEntry, verbose: bool) -> Result<(), String> {
    let lib_root = format!("{dir}/libs/{}", entry.id);
    if is_installed(dir, &entry.id) {
        // 自愈：若上次只写了空壳（0 bin 0 lib），说明那次安装失败，删除重装
        match get_meta(dir, &entry.id) {
            Ok(m) if m.bins.is_empty() && m.libs.is_empty() => {
                util::log_file(
                    dir,
                    &format!("检测到 {} 安装为空壳（无 bin/lib），删除并重装", entry.id),
                );
                let _ = std::fs::remove_dir_all(&lib_root);
            }
            _ => {
                if verbose {
                    println!("已安装过: {}，可先 remove 再重装", entry.id);
                }
                return Ok(());
            }
        }
    }

    let _ = fs::create_dir_all(&lib_root);
    let tmp = format!("{dir}/libs/.tmp/{}", entry.id);
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    let meta = match entry.r#type.as_str() {
        "termux" | "bundled" => install_from_termux(dir, entry, &tmp, verbose)?,
        "url" => install_from_url(entry, &tmp, verbose)?,
        other => return Err(format!("未知安装类型: {other}")),
    };

    // 定位 usr 根目录
    let usr = find_usr_root(&tmp);
    let bin_src = format!("{usr}/bin");
    let lib_src = format!("{usr}/lib");
    let bin_dst = format!("{lib_root}/bin");
    let lib_dst = format!("{lib_root}/lib");
    fs::create_dir_all(&bin_dst).map_err(|e| e.to_string())?;
    fs::create_dir_all(&lib_dst).map_err(|e| e.to_string())?;

    let mut installed_bins = Vec::new();
    if Path::new(&bin_src).is_dir() {
        for name in entries(&bin_src)? {
            let from = format!("{bin_src}/{name}");
            if let Ok(md) = fs::symlink_metadata(&from) {
                if md.file_type().is_file() || md.file_type().is_symlink() {
                    let _ = fs::copy(&from, format!("{bin_dst}/{name}"));
                    installed_bins.push(name);
                }
            }
        }
    }
    let mut installed_libs = Vec::new();
    if Path::new(&lib_src).is_dir() {
        for name in entries(&lib_src)? {
            let from = format!("{lib_src}/{name}");
            if name.ends_with(".so") || name.contains(".so.") {
                let _ = fs::copy(&from, format!("{lib_dst}/{name}"));
                installed_libs.push(name);
            }
        }
    }

    let bins = merge_unique(&installed_bins, &meta.bins);
    for b in &bins {
        let _ = util::run(&format!("chmod 755 '{bin_dst}/{b}'"), None);
    }

    // 写 meta.json
    let meta_obj = Json::Obj({
        let mut m = BTreeMap::new();
        m.insert("version".to_string(), Json::Str(meta.version.clone()));
        m.insert("bins".to_string(), Json::Arr(bins.iter().map(|x| Json::Str(x.clone())).collect()));
        m.insert("libs".to_string(), Json::Arr(installed_libs.iter().map(|x| Json::Str(x.clone())).collect()));
        m.insert("source_type".to_string(), Json::Str(meta.source_type.clone()));
        m
    });
    let meta_json = json::to_string_pretty(&meta_obj);
    fs::write(format!("{lib_root}/meta.json"), &meta_json).map_err(|e| e.to_string())?;

    // 记录安装结果（供日志排查：bin/lib 是否为空；空则列出 tmp 目录定位解压问题）
    util::log_file(
        dir,
        &format!("install({}): bins=[{}] libs=[{}]", entry.id, bins.join(","), installed_libs.join(",")),
    );
    if bins.is_empty() && installed_libs.is_empty() {
        util::log_file(dir, &format!("警告: {} 未装出任何文件，tmp 目录结构:", entry.id));
        if let Ok((_, out, _)) = util::run(&format!("ls -laR '{tmp}' 2>/dev/null | head -n 40"), Some(10)) {
            util::log_file(dir, &out);
        }
    }

    let _ = fs::remove_dir_all(&tmp);

    if verbose {
        println!(
            "安装完成: {} v{}（{} 个可执行文件, {} 个动态库）",
            entry.name,
            meta.version,
            bins.len(),
            installed_libs.len()
        );
    }
    Ok(())
}

fn entries(dir: &str) -> Result<Vec<String>, String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .map(|r| r.file_name().to_string_lossy().to_string())
        .collect();
    v.sort();
    Ok(v)
}

fn merge_unique(a: &[String], b: &[String]) -> Vec<String> {
    let mut v = a.to_vec();
    for x in b {
        if !v.contains(x) {
            v.push(x.clone());
        }
    }
    v
}

fn find_usr_root(tmp: &str) -> String {
    let candidates = [
        format!("{tmp}/usr"),
        format!("{tmp}/data/data/com.termux/files/usr"),
    ];
    for c in &candidates {
        if Path::new(c).is_dir() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

/// 从 Termux 仓库安装
fn install_from_termux(
    dir: &str,
    entry: &RepoEntry,
    tmp: &str,
    verbose: bool,
) -> Result<InstalledMeta, String> {
    let pkg = if entry.pkg.is_empty() {
        entry.id.clone()
    } else {
        entry.pkg.clone()
    };
    let cfg = util::load_config(dir)?;
    let mirror = cfg.mirror.trim_end_matches('/').to_string();
    let arch = cfg.arch.clone();

    let cache = format!("{dir}/libs/.cache");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;

    let idx_xz = format!("{cache}/Packages.{arch}.xz");
    let idx_txt = format!("{cache}/Packages.{arch}");
    let idx_url = format!("{mirror}/dists/stable/main/binary-{arch}/Packages.xz");
    if verbose {
        eprintln!("下载仓库索引…");
    }
    util::download(&idx_url, &idx_xz).or_else(|_| {
        let alt = format!("{mirror}/dists/stable/main/binary-{arch}/Packages");
        util::download(&alt, &idx_txt)
    })?;

    if Path::new(&idx_xz).exists() {
        util::xz_decompress(&idx_xz, &idx_txt)?;
    }

    let (version, filename) = parse_packages(&idx_txt, &pkg)?;
    if verbose {
        eprintln!("{} -> {} ({})", entry.name, version, filename);
    }

    let deb_path = format!("{cache}/{}.deb", entry.id);
    let deb_url = format!("{mirror}/{filename}");
    util::download(&deb_url, &deb_path)?;
    util::log_file(
        dir,
        &format!(
            "已下载 .deb: {}（{} 字节）",
            entry.id,
            std::fs::metadata(&deb_path).map(|x| x.len()).unwrap_or(0)
        ),
    );

    // 5) 从 .deb 提取 data 归档（支持 xz / gz / zst）
    let mut data_file = String::new();
    let mut data_kind = "";
    for ext in ["xz", "gz", "zst"] {
        let m = format!("{tmp}/data.tar.{ext}");
        if ar::extract_member(&deb_path, &format!("data.tar.{ext}"), &m).is_ok()
            && std::fs::metadata(&m).map(|x| x.len() > 0).unwrap_or(false)
        {
            data_file = m;
            data_kind = ext;
            break;
        }
    }
    if data_file.is_empty() {
        return Err("无法从 .deb 提取 data 归档（xz/gz/zst 均未找到）".to_string());
    }
    util::log_file(
        dir,
        &format!(
            "已提取 data.tar.{data_kind}（{} 字节）",
            std::fs::metadata(&data_file).map(|x| x.len()).unwrap_or(0)
        ),
    );

    let extract_cmd = if data_kind == "xz" {
        let data_tar = format!("{tmp}/data.tar");
        util::xz_decompress(&data_file, &data_tar)?;
        format!("cd '{tmp}' && tar -xf '{data_tar}' 2>/dev/null || busybox tar -xf '{data_tar}'")
    } else {
        // gz/zst：tar 自动识别压缩格式（zst 需 tar 支持，尽力而为）
        format!("cd '{tmp}' && tar -xf '{data_file}' 2>/dev/null || busybox tar -xf '{data_file}'")
    };
    let (ok, _, err) = util::run(&extract_cmd, Some(120))?;
    if !ok {
        return Err(format!("解包 data.tar.{data_kind} 失败: {err}"));
    }

    Ok(InstalledMeta {
        version,
        bins: Vec::new(),
        libs: Vec::new(),
        source_type: "termux".to_string(),
    })
}

fn parse_packages(packages: &str, pkg: &str) -> Result<(String, String), String> {
    let text = fs::read_to_string(packages).map_err(|e| format!("读取索引失败: {e}"))?;
    let mut cur_pkg = String::new();
    let mut version = String::new();
    let mut filename = String::new();
    let mut found = false;

    for line in text.lines() {
        if line.is_empty() {
            if found {
                if filename.is_empty() {
                    return Err("索引中该包缺少 Filename".to_string());
                }
                return Ok((version, filename));
            }
            cur_pkg.clear();
            version.clear();
            filename.clear();
            continue;
        }
        if let Some(v) = line.strip_prefix("Package: ") {
            cur_pkg = v.trim().to_string();
            if cur_pkg == pkg {
                found = true;
            }
            continue;
        }
        if found {
            if let Some(v) = line.strip_prefix("Version: ") {
                version = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Filename: ") {
                filename = v.trim().to_string();
            }
        }
    }
    if found {
        return Ok((version, filename));
    }
    Err(format!("仓库索引中找不到包: {pkg}"))
}

fn install_from_url(entry: &RepoEntry, tmp: &str, verbose: bool) -> Result<InstalledMeta, String> {
    let file = format!("{tmp}/download");
    if verbose {
        eprintln!("下载 {} …", entry.url);
    }
    util::download(&entry.url, &file)?;

    let archive = if entry.archive.is_empty() {
        "raw"
    } else {
        entry.archive.as_str()
    };

    match archive {
        "raw" => {
            fs::create_dir_all(format!("{tmp}/usr/bin")).map_err(|e| e.to_string())?;
            let dst = format!("{tmp}/usr/bin/{}", entry.bins.first().map(|s| s.as_str()).unwrap_or("app"));
            fs::copy(&file, &dst).map_err(|e| e.to_string())?;
            Ok(InstalledMeta {
                version: entry.version.clone(),
                bins: entry.bins.clone(),
                libs: entry.libs.clone(),
                source_type: "url".to_string(),
            })
        }
        "zip" => {
            let (ok, _, err) = util::run(&format!("cd '{tmp}' && unzip -o -q '{file}'"), Some(180))?;
            if !ok {
                return Err(format!("解压 zip 失败: {err}"));
            }
            Ok(InstalledMeta {
                version: entry.version.clone(),
                bins: entry.bins.clone(),
                libs: entry.libs.clone(),
                source_type: "url".to_string(),
            })
        }
        "tar.gz" | "tgz" => {
            let (ok, _, err) = util::run(&format!("cd '{tmp}' && tar -xzf '{file}'"), Some(180))?;
            if !ok {
                return Err(format!("解压 tar.gz 失败: {err}"));
            }
            Ok(InstalledMeta {
                version: entry.version.clone(),
                bins: entry.bins.clone(),
                libs: entry.libs.clone(),
                source_type: "url".to_string(),
            })
        }
        "tar.xz" => {
            let (ok, _, err) = util::run(&format!("cd '{tmp}' && tar -xJf '{file}'"), Some(180))?;
            if !ok {
                return Err(format!("解压 tar.xz 失败: {err}"));
            }
            Ok(InstalledMeta {
                version: entry.version.clone(),
                bins: entry.bins.clone(),
                libs: entry.libs.clone(),
                source_type: "url".to_string(),
            })
        }
        other => Err(format!("不支持的归档类型: {other}")),
    }
}

/// list 命令：合并仓库与本地状态，输出完整 JSON
pub fn cmd_list(dir: &str) -> Result<(), String> {
    let repos = load_repos(dir)?;
    let st = load_state(dir)?;
    let libs_map = st.get("libs").and_then(|v| match v {
        Json::Obj(m) => Some(m),
        _ => None,
    });

    let mut items = Vec::new();
    for r in &repos {
        let installed = is_installed(dir, &r.id);
        let mounted = libs_map
            .and_then(|m| m.get(&r.id))
            .and_then(|v| match v {
                Json::Obj(ref o) => o.get("mounted").and_then(|x| match x {
                    Json::Bool(b) => Some(*b),
                    _ => None,
                }),
                _ => None,
            })
            .unwrap_or(false);
        let version = if installed {
            get_meta(dir, &r.id).map(|m| m.version).unwrap_or_default()
        } else {
            String::new()
        };

        let item = Json::Obj({
            let mut m = BTreeMap::new();
            m.insert("id".to_string(), Json::Str(r.id.clone()));
            m.insert("name".to_string(), Json::Str(r.name.clone()));
            m.insert("desc".to_string(), Json::Str(r.desc.clone()));
            m.insert("category".to_string(), Json::Str(r.category.clone()));
            m.insert("type".to_string(), Json::Str(r.r#type.clone()));
            m.insert("core".to_string(), Json::Bool(r.core));
            m.insert("installed".to_string(), Json::Bool(installed));
            m.insert("mounted".to_string(), Json::Bool(mounted));
            m.insert("version".to_string(), Json::Str(version));
            m
        });
        items.push(item);
    }
    println!("{}", json::to_string(&Json::Arr(items)));
    Ok(())
}

/// status 命令：简要状态
pub fn cmd_status(dir: &str) -> Result<(), String> {
    let repos = load_repos(dir)?;
    let st = load_state(dir)?;
    let libs_map = st.get("libs").and_then(|v| match v {
        Json::Obj(m) => Some(m),
        _ => None,
    });
    let installed = repos.iter().filter(|r| is_installed(dir, &r.id)).count();
    let mounted = libs_map
        .map(|m| m.values().filter(|v| match v {
            Json::Obj(ref o) => o.get("mounted").and_then(|x| match x {
                Json::Bool(b) => Some(*b),
                _ => None,
            }).unwrap_or(false),
            _ => false,
        }).count())
        .unwrap_or(0);
    let core_ok = repos.iter().filter(|r| r.core && is_installed(dir, &r.id)).count();
    let core_total = repos.iter().filter(|r| r.core).count();
    let arch = util::load_config(dir).map(|c| c.arch).unwrap_or_else(|_| "aarch64".to_string());

    let out = Json::Obj({
        let mut m = BTreeMap::new();
        m.insert("total".to_string(), Json::Num(repos.len().to_string()));
        m.insert("installed".to_string(), Json::Num(installed.to_string()));
        m.insert("mounted".to_string(), Json::Num(mounted.to_string()));
        m.insert("core_ok".to_string(), Json::Num(core_ok.to_string()));
        m.insert("core_total".to_string(), Json::Num(core_total.to_string()));
        m.insert("arch".to_string(), Json::Str(arch));
        m
    });
    println!("{}", json::to_string(&out));
    Ok(())
}