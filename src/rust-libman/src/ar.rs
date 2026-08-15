//! ar 归档解析（用于解包 .deb）
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
//! .deb 本质是 ar 归档，内含 debian-binary / control.tar.* / data.tar.*
//! 只需取出指定成员写入文件即可。

use std::io::{Read, Seek, SeekFrom, Write};

const AR_MAGIC: &[u8; 8] = b"!<arch>\n";
const HEADER_LEN: u64 = 60;
const FILE_MAGIC: &[u8; 2] = b"`\n";

/// 从 ar 归档路径中提取名为 `member` 的成员到 `out_path`。
/// member 名称按前缀匹配（去掉结尾 '/' 与长名表）。
pub fn extract_member(archive: &str, member: &str, out_path: &str) -> Result<(), String> {
    let mut f = std::fs::File::open(archive).map_err(|e| format!("打开 {archive}: {e}"))?;

    // 校验魔数
    let mut magic = [0u8; 8];
    f.read_exact(&mut magic)
        .map_err(|e| format!("读取 ar 魔数: {e}"))?;
    if &magic != AR_MAGIC {
        return Err("不是有效的 ar 归档".to_string());
    }

    loop {
        let mut header = [0u8; HEADER_LEN as usize];
        match f.read_exact(&mut header) {
            Ok(_) => {}
            Err(_) => break, // 到末尾
        }
        // 名字: 16 字节，通常是 "name/"（结尾斜杠）
        let raw_name = String::from_utf8_lossy(&header[0..16])
            .trim_end_matches(char::from(0))
            .trim_end()
            .to_string();
        let name = raw_name.trim_end_matches('/').to_string();

        // 大小: offset 48..58，十进制
        let size_str = String::from_utf8_lossy(&header[48..58])
            .trim()
            .to_string();
        let size: u64 = size_str.parse().unwrap_or(0);

        // 校验尾魔数
        if &header[58..60] != FILE_MAGIC {
            // 非标准头（如 GNU 长名表），保守处理：跳过
            f.seek(SeekFrom::Current(size as i64))
                .map_err(|e| e.to_string())?;
            continue;
        }

        if name == member {
            let mut data = vec![0u8; size as usize];
            f.read_exact(&mut data).map_err(|e| format!("读取成员 {member}: {e}"))?;
            let mut out = std::fs::File::create(out_path)
                .map_err(|e| format!("写入 {out_path}: {e}"))?;
            out.write_all(&data).map_err(|e| e.to_string())?;
            return Ok(());
        }

        // 跳过该成员（不足 2 字节对齐则补）
        let mut skip = size;
        if size % 2 == 1 {
            skip += 1;
        }
        f.seek(SeekFrom::Current(skip as i64))
            .map_err(|e| e.to_string())?;
    }

    Err(format!("ar 中未找到成员: {member}"))
}
