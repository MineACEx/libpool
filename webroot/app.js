/**
 * LibPool · 库池 — KsuWebUI 应用逻辑
 * Copyright (C) 2026 MINO · Himer (MineACE)
 * SPDX-License-Identifier: Apache-2.0
 * 通过 KernelSU 注入的 ksu 桥（经 kernelsu.js）与模块内的 libman 工具通信。
 *
 * 设计要点（触摸优先）：
 * - 涟漪 / 按压反馈（无 hover，针对手机平板）
 * - 公告：优先读模块 .git/url.txt（用户可手动填写），否则回退 ANNOUNCEMENT_URL；
 *   纯文本站点，毛玻璃弹窗 + 背景丝滑渐变模糊
 * - 自定义背景图（支持粘贴网址 + 本地相册选择压缩）、上下边框模糊过渡、主题切换丝滑遮罩
 * - 搜索防抖 + 重建列表跳过入场动画、下载进度用不确定光条（不造假百分比）
 * - 滚动入场（IntersectionObserver，逐个滑入，先快后慢）
 * - 下载可取消（pkill 对应 install 进程）
 */

import {
  exec, toast, moduleInfo, fullScreen, enableEdgeToEdge,
} from "./kernelsu.js";

/* ---------------- 全局状态 ---------------- */
const state = {
  info: { id: "libpool", name: "LibPool" },
  repos: [],          // 仓库列表
  libs: [],           // libman list 输出（含 installed/mounted/version）
  query: "",
  category: "全部",
  mirror: "",
  inPreview: false,
  cancelRequested: false, // 下载取消标记
};

/* 公告网址（指向本仓库 announce.md，内容为 Markdown，每天最多弹一次）。
   用 jsDelivr CDN 走国内节点，访问更快；留空则不启用公告。
   设备上优先读模块 .git/url.txt（用户可手动覆盖）。 */
const ANNOUNCEMENT_URL = "https://cdn.jsdelivr.net/gh/MineACEx/libpool@master/announce.md";

/* 云更新配置（与公告同机制：可读模块 .git/update.txt、.git/download.txt，否则回退内置常量）。
   UPDATE_URL 指向纯文本版本号（如 1.1.0）；UPDATE_DOWNLOAD_URL 指向下载/更新页面。
   检测到云端版本与本地不一致时，弹窗提示并给出可点击跳转默认浏览器的下载链接。
   DEFAULT_RELEASES_URL：未配置下载地址时的兜底更新页。 */
const UPDATE_URL = "https://cdn.jsdelivr.net/gh/MineACEx/libpool@master/version.txt";
const UPDATE_DOWNLOAD_URL = "https://github.com/MineACEx/libpool/releases";

/* ---------------- 模块路径与 libman 调用 ---------------- */
let MOD = "/data/adb/modules/libpool";
let useNative = false;

async function detectEnvironment() {
  try {
    state.info = moduleInfo();
    if (state.info && state.info.id) MOD = `/data/adb/modules/${state.info.id}`;
    // 浏览器预览（无 ksu 桥）：直接进入预览模式
    if (window.__kernelsu && window.__kernelsu.isMock) {
      state.inPreview = true;
      console.warn("非 KernelSU 环境，进入预览模式");
      return;
    }
    await normalizeModule(); // 自愈：规整旧包遗留的反斜杠文件名（见下）
    await ensureToolsExec(); // 自愈：强制补 tools +x 权限并重测（见下）
  } catch (e) {
    state.inPreview = true;
    console.warn("非 KernelSU 环境，进入预览模式", e);
  }
}

/** 自愈：早期坏包（Windows .NET ZipFile 打包）可能留下 webroot\style.css 这类
    带反斜杠字面文件名的文件，导致 tools/libman.sh 等路径检测不到、提示"管理工具未就绪"。
    每次启动只读扫描一次，发现才规整为正斜杠并重新探测工具，无需重装模块。 */
async function normalizeModule() {
  if (state.inPreview || !MOD) return;
  try {
    const scan = await exec(`find ${MOD} -name '*\\\\*' 2>/dev/null`);
    const bad = (scan.stdout || "").split(/\r?\n/).filter(Boolean);
    if (!bad.length) return;
    logWebui(`检测到 ${bad.length} 个反斜杠文件名（旧包遗留），正在规整…`);
    await exec(`find ${MOD} -name '*\\\\*' 2>/dev/null | while IFS= read -r f; do g=$(printf '%s' "$f" | tr '\\\\' '/'); mkdir -p "\${g%/*}" 2>/dev/null; mv -f "$f" "$g" 2>/dev/null; done`);
    const r = await exec(`test -x ${MOD}/tools/libman && echo yes`);
    useNative = /yes/.test(r.stdout || "");
    logWebui("反斜杠文件已规整，已重新探测管理工具");
  } catch (e) { /* 自愈失败不阻断启动 */ }
}

/** 自愈：确保管理工具可用。
    zip 由 Windows 打包时通常不带 Unix +x 位，管理器解压后 tools/* 全是 644，
    test -x 会把 libman 误判成"管理工具未就绪"（服务/预检/诊断三处都报不可执行，
    但用 sh 直接跑其实正常）。WebUI 的 exec 以 root 运行，这里先强制 chmod 755。
    之后原生优先（Rust 版 libman 已带完整日志 logs/libman.log），缺失再回退 shell 兜底。 */
async function ensureToolsExec() {
  if (state.inPreview || !MOD) return;
  try {
    await exec(`chmod 755 ${MOD}/tools 2>/dev/null; chmod 755 ${MOD}/tools/* 2>/dev/null`);
    // 原生优先，缺失则 shell 兜底
    const r = await exec(`test -x ${MOD}/tools/libman && echo native || test -x ${MOD}/tools/libman.sh && echo shell`);
    const m = /(native|shell)/.exec(r.stdout || "");
    useNative = m ? m[1] === "native" : false;
    logWebui(`工具就绪: ${useNative ? "libman（Rust 原生）" : "libman.sh（shell 兜底）"}`);
  } catch (e) { /* 自愈失败不阻断启动 */ }
}

/** 构造 libman 命令（shell 前缀） */
function libmanCmd(args) {
  const tool = useNative ? `${MOD}/tools/libman` : `sh ${MOD}/tools/libman.sh`;
  return `LIBPOOL_DIR=${MOD} ${tool} ${args}`;
}

async function libmanExec(args) {
  const r = await exec(libmanCmd(args));
  if (r.errno !== 0) throw new Error((r.stderr || "执行失败").trim());
  return r.stdout || "";
}

/* ---------------- 数据加载 ---------------- */
async function loadRepos() {
  try {
    const res = await fetch("./data/repos.json", { cache: "no-store" });
    state.repos = await res.json();
  } catch (e) {
    console.error("repos.json 加载失败", e);
    state.repos = [];
  }
}

async function loadLibs() {
  if (state.inPreview) {
    // 预览模式：全部标为"未安装"
    state.libs = state.repos.map((r) => ({
      id: r.id, name: r.name, desc: r.desc, category: r.category,
      type: r.type, core: !!r.core, installed: false, mounted: false, version: "",
    }));
    return;
  }
  try {
    const out = await libmanExec("list");
    const parsed = JSON.parse(out);
    if (Array.isArray(parsed)) state.libs = parsed;
  } catch (e) {
    console.error(e);
    state.libs = [];
  }
}

async function loadMirror() {
  if (state.inPreview) {
    $("aboutArch").textContent = "预览模式";
    return;
  }
  try {
    const out = await libmanExec("config");
    const cfg = JSON.parse(out);
    state.mirror = cfg.mirror || "";
  } catch (e) { /* ignore */ }
  // 显示设备架构（32 位兼容性一目了然）
  try {
    const u = await exec("uname -m");
    const arch = (u.stdout || "").trim();
    if (arch) $("aboutArch").textContent = arch;
  } catch (e) { /* ignore */ }
}

/* ---------------- DOM 引用 ---------------- */
const $ = (id) => document.getElementById(id);
const mountedListEl = $("mountedList");
const storeGridEl = $("storeGrid");
const catChipsEl = $("catChips");

/* ---------------- 渲染：已挂载 ---------------- */
function renderMounted(animate = true) {
  const mounted = state.libs.filter((l) => l.installed);
  animateNumber($("statMounted"), state.libs.filter((l) => l.mounted).length);
  animateNumber($("statInstalled"), mounted.length);
  animateNumber($("statTotal"), state.repos.length);

  if (mounted.length === 0) {
    mountedListEl.innerHTML = "";
    $("mountedHint").hidden = false;
    return;
  }
  $("mountedHint").hidden = true;
  mountedListEl.innerHTML = mounted
    .map((l, i) => libCardHTML(l, i, true, animate))
    .join("");
  // 绑定开关
  mountedListEl.querySelectorAll(".switch").forEach((sw) => {
    sw.addEventListener("click", () => toggleLib(sw.dataset.id));
  });
  // 绑定删除
  mountedListEl.querySelectorAll(".btn-remove").forEach((b) => {
    b.addEventListener("click", () => removeLib(b.dataset.id));
  });
}

function iconClass(cat) {
  if (/编程|语言|解释|开发/.test(cat)) return "purple";
  if (/压缩|打包/.test(cat)) return "orange";
  if (/网络|下载|服务器/.test(cat)) return "";
  if (/文本|编辑|终端/.test(cat)) return "green";
  return "gray";
}

function libCardHTML(l, i, isMounted, animate = true) {
  const color = iconClass(l.category);
  const reveal = animate ? "reveal" : "reveal in";   // 滚动入场：animate=false 直接显示
  const idx = animate ? `style="--i:${Math.min(i, 12)}"` : "";
  return `
  <div class="lib-card ${reveal}" ${idx}>
    <div class="lib-icon ${color}">${(l.name || "?")[0].toUpperCase()}</div>
    <div class="lib-body">
      <div class="lib-name">
        <span>${escapeHtml(l.name)}</span>
        ${l.version ? `<span class="lib-ver">${escapeHtml(l.version)}</span>` : ""}
        ${l.core ? '<span class="lib-tag core">自带</span>' : ""}
      </div>
      <div class="lib-desc">${escapeHtml(l.desc || "")}</div>
    </div>
    ${isMounted ? `
      <button class="switch ${l.mounted ? "on" : ""}" data-id="${l.id}" role="switch" aria-checked="${!!l.mounted}" aria-label="挂载 ${l.name}"></button>
    ` : `
      <button class="btn-remove ripple-host" data-id="${l.id}">删除</button>
    `}
  </div>`;
}

/* ---------------- 渲染：扩展库 ---------------- */
function renderStore() {
  const cats = ["全部", ...new Set(state.repos.map((r) => r.category).filter(Boolean))];
  catChipsEl.innerHTML = cats
    .map((c) => `<button class="chip ${c === state.category ? "active" : ""}" data-cat="${c}">${escapeHtml(c)}</button>`)
    .join("");
  catChipsEl.querySelectorAll(".chip").forEach((c) => {
    c.addEventListener("click", () => {
      state.category = c.dataset.cat;
      catChipsEl.querySelectorAll(".chip").forEach((x) => x.classList.toggle("active", x === c));
      renderStoreGrid(false); // 切分类：跳过入场 stagger
    });
  });

  renderStoreGrid();
}

function renderStoreGrid(animate = true) {
  const q = state.query.trim().toLowerCase();
  const list = state.repos.filter((r) => {
    if (state.category !== "全部" && r.category !== state.category) return false;
    if (q && !((r.name + " " + (r.desc || "") + " " + r.id).toLowerCase().includes(q))) return false;
    return true;
  });

  $("storeEmpty").hidden = list.length > 0;
  storeGridEl.innerHTML = list.map((r, i) => storeCardHTML(r, i, animate)).join("");

  storeGridEl.querySelectorAll("[data-action]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const action = btn.dataset.action;
      const id = btn.dataset.id;
      if (action === "download") installLib(id);
      else if (action === "remove") removeLib(id);
      else if (action === "toggle") toggleLib(id);
    });
  });
}

function storeCardHTML(r, i, animate = true) {
  const local = state.libs.find((l) => l.id === r.id);
  const installed = local ? local.installed : false;
  const mounted = local ? local.mounted : false;
  const version = local ? local.version : "";
  const color = iconClass(r.category);
  const reveal = animate ? "reveal" : "reveal in";   // 滚动入场
  const idx = animate ? `style="--i:${Math.min(i, 12)}"` : "";
  let foot = `<button class="btn-download ripple-host" data-action="download" data-id="${r.id}">下载</button>`;
  if (installed) {
    foot = `
      <button class="switch ${mounted ? "on" : ""}" data-action="toggle" data-id="${r.id}" aria-checked="${mounted}" aria-label="挂载 ${r.name}"></button>
      <button class="btn-remove ripple-host" data-action="remove" data-id="${r.id}">删除</button>
    `;
  }
  return `
  <div class="store-card ${reveal}" ${idx}>
    <div class="store-head">
      <div class="lib-icon ${color}">${(r.name || "?")[0].toUpperCase()}</div>
      <div class="lib-body">
        <div class="lib-name" style="font-size:14.5px">
          ${escapeHtml(r.name)}
          ${r.core ? '<span class="lib-tag core">自带</span>' : ""}
        </div>
        ${version ? `<div class="lib-ver">${escapeHtml(version)}</div>` : ""}
      </div>
    </div>
    <div class="store-body">
      <div class="store-desc">${escapeHtml(r.desc || "")}</div>
    </div>
    <div class="store-foot">${foot}</div>
  </div>`;
}

/* ---------------- 操作 ---------------- */
async function toggleLib(id) {
  if (state.inPreview) { showToast("预览模式下不执行操作"); return; }
  logWebui(`toggle: ${id}`);
  try {
    const r = await exec(libmanCmd(`toggle ${id}`));
    if (r.errno !== 0) throw new Error((r.stderr || "操作失败").trim());
    showToast((r.stdout || "已切换").trim());
    await refresh();
  } catch (e) {
    logWebui(`toggle 失败: ${id} ${e.message}`);
    showToast("操作失败：" + e.message, true);
    console.error(e);
  }
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function installLib(id) {
  if (state.inPreview) { showToast("预览模式下不执行操作"); return; }
  const lib = state.repos.find((r) => r.id === id);
  // 预检：确认管理工具就绪，并记录真实原因到日志（避免"秒弹又秒关"的困惑）。
  try {
    const probe = await exec(
      `chmod 755 ${MOD}/tools 2>/dev/null; chmod 755 ${MOD}/tools/* 2>/dev/null; ` +
      `ls -l ${MOD}/tools/ 2>&1; echo ---; ` +
      `test -x ${MOD}/tools/libman && echo TOOL_NATIVE || echo TOOL_NO_NATIVE; ` +
      `test -x ${MOD}/tools/libman.sh && echo TOOL_SHELL || echo TOOL_NO_SHELL`
    );
    logWebui(`工具预检(${id}):\n${probe.stdout || ""}`);
    if (!/TOOL_NATIVE|TOOL_SHELL/.test(probe.stdout || "")) {
      // 最后一层兜底：即使 +x 位拿不到，只要 sh 能跑 shell 版就继续（sh 不要求 +x）
      const v = await exec(`sh ${MOD}/tools/libman.sh version 2>&1`);
      if (/libman/.test(v.stdout || "")) {
        useNative = false;
        logWebui("libman.sh 无 +x 位但可经 sh 运行，已改用 shell 兜底继续");
      } else {
        // 写诊断日志并提示用户去查看（置顶有架构/tools 权限/可执行性，直接说明原因）
        const diag = await runDiagnostics();
        logWebui(`工具未就绪，诊断：\n${diag}`);
        showToast("管理工具未就绪，请到「设置 → 查看日志」看原因", true);
        return;
      }
    }
  } catch (e) { /* 预检失败不阻断，由下方正式执行兜底 */ }
  logWebui(`开始下载: ${id}`);
  state.cancelRequested = false;
  showOverlay(`正在安装 ${lib ? lib.name : id}`, "后台安装中，可稍等或取消…");
  try {
    // 后台启动安装：exec 只负责拉起 nohup 进程、立即返回，然后 WebUI 轮询判断完成。
    // 不能直接 exec 等长命令——KsuWebUI 的 exec 会同步等待命令结束，16 秒+的下载
    // 会把 WebUI 主线程卡死（曾出现点下载后整页卡死）。轮询用 kill -0 / cat 等毫秒级命令，无感。
    const tool = useNative ? `${MOD}/tools/libman` : `sh ${MOD}/tools/libman.sh`;
    const cmd = `LIBPOOL_DIR=${MOD} ${tool} install ${id}`;
    const pidPath = `${MOD}/logs/.install-${id}.pid`;
    const logPath = `${MOD}/logs/install-${id}.log`;
    await exec(`mkdir -p ${MOD}/logs; rm -f ${pidPath} ${logPath}; nohup sh -c '${cmd}' > ${logPath} 2>&1 & echo $! > ${pidPath}`);

    // 轮询后台进程是否结束（每 1.5s 一次，超时 240 秒）
    let pid = ((await exec(`cat ${pidPath} 2>/dev/null`)).stdout || "").trim();
    const t0 = Date.now();
    while (true) {
      if (state.cancelRequested) {
        await exec(`kill ${pid} 2>/dev/null; pkill -f "libman install ${id}" 2>/dev/null; true`);
        throw new Error("已取消");
      }
      if (Date.now() - t0 > 240000) {
        await exec(`kill ${pid} 2>/dev/null; pkill -f "libman install ${id}" 2>/dev/null; true`);
        throw new Error("安装超时（240 秒），请查看日志");
      }
      await sleep(1500);
      const alive = pid
        ? await exec(`kill -0 ${pid} 2>/dev/null && echo YES || echo NO`)
        : await exec(`pgrep -f "libman install ${id}" >/dev/null 2>&1 && echo YES || echo NO`);
      if ((alive.stdout || "").trim() !== "YES") break;
    }

    // 读取本次安装日志判断结果
    const r = await exec(`cat ${logPath} 2>/dev/null`);
    const out = r.stdout || "";
    logWebui(`安装日志(${id}):\n${out.slice(0, 900)}`);
    if (/libman 错误|命令失败|索引中找不到|所有镜像均无法|下载失败/.test(out)) {
      const m = out.match(/libman 错误:([^\n]*)/);
      throw new Error((m ? m[1].trim() : "安装失败，请查看日志").slice(0, 120));
    }
    hideOverlay();
    logWebui(`下载完成: ${id}`);
    await verifyInstall(id); // 校验安装产物（bin/lib 是否真的装出了文件）
    showToast("下载完成");
    await refresh();
    pulseCard(id); // 安装成功：对应卡片脉冲光晕（纯视觉）
  } catch (e) {
    hideOverlay();
    logWebui(`下载失败: ${id} ${e.message}`);
    if (!state.cancelRequested) showToast("下载失败：" + e.message, true);
    console.error(e);
  }
}

/** 校验安装产物：检查 libs/<id>/bin 与 lib 是否真的装出了文件。
    若为空说明下载/解压/搬运某一步出问题，提示并记录，避免"假成功"。 */
async function verifyInstall(id) {
  if (state.inPreview || !MOD) return;
  try {
    const ls = await exec(`ls -l ${MOD}/libs/${id}/bin ${MOD}/libs/${id}/lib 2>&1`);
    logWebui(`安装产物(${id}):\n${ls.stdout || ""}`);
    const has = await exec(`find ${MOD}/libs/${id}/bin ${MOD}/libs/${id}/lib -type f 2>/dev/null | head -n 1`);
    if (!(has.stdout || "").trim()) {
      logWebui(`警告: ${id} 安装后 bin/lib 为空，可能下载/解压失败，详见 logs/libman.log`);
      showToast("下载完成，但 bin/lib 为空，请查日志", true);
    }
  } catch (e) { /* ignore */ }
}

async function cancelDownload() {
  if (!state.cancelRequested) {
    state.cancelRequested = true;
    try { await exec(`pkill -f "libman install"`); } catch (e) { /* ignore */ }
  }
  hideOverlay();
  showToast("已取消下载");
}

async function removeLib(id) {
  if (state.inPreview) { showToast("预览模式下不执行操作"); return; }
  logWebui(`remove: ${id}`);
  try {
    const r = await exec(libmanCmd(`remove ${id}`));
    if (r.errno !== 0) throw new Error((r.stderr || "删除失败").trim());
    showToast((r.stdout || "已删除").trim());
    await refresh();
  } catch (e) {
    logWebui(`remove 失败: ${id} ${e.message}`);
    showToast("删除失败：" + e.message, true);
  }
}

async function refresh() {
  await loadLibs();
  renderMounted(false);
  renderStoreGrid(false);
  renderStatus();
}

/* ---------------- 状态徽章 ---------------- */
function renderStatus() {
  const chip = $("statusChip");
  const text = $("statusText");
  if (state.inPreview) {
    chip.className = "status-chip off";
    text.textContent = "预览模式";
    return;
  }
  const coreTotal = state.repos.filter((r) => r.core).length;
  const ok = state.libs.filter((l) => l.installed && l.core).length;
  if (coreTotal === 0) {
    chip.className = "status-chip";
    text.textContent = `已就绪 · ${state.libs.filter((l) => l.mounted).length} 个已挂载`;
    return;
  }
  if (ok >= coreTotal) {
    chip.className = "status-chip";
    text.textContent = `已就绪 · ${state.libs.filter((l) => l.mounted).length} 个已挂载`;
  } else {
    chip.className = "status-chip warn";
    text.textContent = `核心库就绪 ${ok}/${coreTotal}`;
  }
}

/* ---------------- Tab 切换（指示条滑动 + 面板切换，无模糊遮罩） ---------------- */
function setupTabs() {
  document.querySelectorAll(".tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      const name = tab.dataset.tab;
      document.querySelectorAll(".tab").forEach((t) => {
        const on = t === tab;
        t.classList.toggle("active", on);
        t.setAttribute("aria-selected", on);
      });
      document.querySelectorAll(".panel").forEach((p) => {
        const on = p.id === `panel-${name}`;
        p.hidden = !on;
        p.classList.toggle("active", on);
        // 强制同步布局：让毛玻璃在显示首帧就完成绘制，避免"晚 0.x 秒才糊出来"
        if (on) void p.offsetHeight;
      });
      document.querySelector(".tabs").dataset.active = name;
      // 面板从 hidden 变为可见，强制重检入场动画（display:none 内 IO 不自动触发）
      if (revealRecheck) setTimeout(revealRecheck, 30);
    });
  });
}

/* ---------------- 主题（丝滑遮罩过渡） ---------------- */
function setupTheme() {
  const saved = localStorage.getItem("libpool-theme") || "auto";
  applyTheme(saved);
  // 只在主题组内切换选中态：不能 querySelectorAll(".seg-btn") 全选，
  // 否则会误清"圆角类型"等其它 seg 组的选中框（只能亮一个的 bug）
  const seg = document.querySelectorAll("#themeSeg .seg-btn");
  seg.forEach((b) => {
    const on = b.dataset.theme === saved;
    b.classList.toggle("active", on);
    b.addEventListener("click", () => {
      applyTheme(b.dataset.theme);
      localStorage.setItem("libpool-theme", b.dataset.theme);
      seg.forEach((x) => x.classList.toggle("active", x === b));
    });
  });
}

function applyTheme(theme) {
  const scrim = $("themeScrim");
  // 并行动画：遮罩淡入 + 模糊升起盖住旧主题（blur 0→36px 过渡 0.55s）→ 切换 → 淡出露出新主题。
  // .on 保持足够久（340ms），让模糊充分升起可见，避免"刚起步就被移除看不到模糊"。
  scrim.classList.add("on");
  setTimeout(() => {
    document.documentElement.dataset.theme = theme;
  }, 150);
  setTimeout(() => {
    requestAnimationFrame(() => scrim.classList.remove("on"));
  }, 340);
}

/* ---------------- 卡片透明度 / 模糊度（设置页滑杆） ----------------
   实时写入 CSS 变量（--card-alpha / --card-blur），所有毛玻璃卡片即时生效；
   值持久化到 localStorage，下次打开仍生效。 */
function setupGlass() {
  const alphaSlider = $("alphaSlider");
  const blurSlider = $("blurSlider");
  const alphaVal = $("alphaVal");
  const blurVal = $("blurVal");

  const savedAlpha = parseFloat(localStorage.getItem("libpool-alpha"));
  const savedBlur = parseInt(localStorage.getItem("libpool-blur"), 10);
  const alpha = Number.isFinite(savedAlpha) ? savedAlpha : 0.55;
  const blur = Number.isFinite(savedBlur) ? savedBlur : 24;

  const apply = () => {
    const a = parseFloat(alphaSlider.value) / 100;
    const b = parseInt(blurSlider.value, 10);
    document.documentElement.style.setProperty("--card-alpha", a);
    document.documentElement.style.setProperty("--card-blur", b + "px");
    alphaVal.textContent = Math.round(a * 100) + "%";
    blurVal.textContent = b;
  };

  alphaSlider.value = Math.round(alpha * 100);
  blurSlider.value = blur;
  apply();

  alphaSlider.addEventListener("input", () => {
    apply();
    localStorage.setItem("libpool-alpha", parseFloat(alphaSlider.value) / 100);
  });
  blurSlider.addEventListener("input", () => {
    apply();
    localStorage.setItem("libpool-blur", blurSlider.value);
  });
}

/* ---------------- 圆角类型 / 大小（设置页） ----------------
   --radius-shape：切换按钮控制（squircle=G2 连续 / round=经典抗锯齿）。
   --r-card：大小滑杆控制（16-48px，默认 32px 中间位），G2/经典两种模式都生效。
   按钮圆角 --r-btn 自适应 --r-card（≤22px），不套 squircle。全部持久化。 */
function setupRadius() {
  const typeBtns = document.querySelectorAll("[data-radius-type]");
  const sizeSlider = $("radiusSizeSlider");
  const sizeVal = $("radiusSizeVal");
  if (!sizeSlider || !sizeVal) return;

  const savedType = localStorage.getItem("libpool-radius-type") || "squircle";
  const savedSize = parseInt(localStorage.getItem("libpool-radius-size"), 10);
  const size = Number.isFinite(savedSize) ? savedSize : 32;

  const applyType = (t) => {
    document.documentElement.style.setProperty("--radius-shape", t);
    typeBtns.forEach((b) => b.classList.toggle("active", b.dataset.radiusType === t));
    localStorage.setItem("libpool-radius-type", t);
  };

  const applySize = () => {
    const n = parseInt(sizeSlider.value, 10);
    // JS 直接计算派生圆角：避免 Blink 对 min()/max() 嵌套 calc() 变量重算的兼容问题
    document.documentElement.style.setProperty("--r-card", n + "px");
    document.documentElement.style.setProperty("--r-btn", Math.max(12, Math.min(n - 4, 20)) + "px");
    document.documentElement.style.setProperty("--r-input", Math.max(14, n - 6) + "px");
    document.documentElement.style.setProperty("--r-sm", Math.max(10, Math.round(n * 0.5)) + "px");
    sizeVal.textContent = n + "px";
    localStorage.setItem("libpool-radius-size", n);
  };

  typeBtns.forEach((b) => b.addEventListener("click", () => applyType(b.dataset.radiusType)));
  sizeSlider.value = size;
  applyType(savedType);
  applySize();
  sizeSlider.addEventListener("input", applySize);
}

/* ---------------- 按钮透明度 / 模糊度（设置页滑杆） ----------------
   实时写入 --btn-alpha / --btn-blur，所有毛玻璃按钮（下载/主按钮/危险按钮等）即时生效；
   持久化到 localStorage。 */
function setupBtnGlass() {
  const aSlider = $("btnAlphaSlider");
  const bSlider = $("btnBlurSlider");
  const aVal = $("btnAlphaVal");
  const bVal = $("btnBlurVal");
  if (!aSlider || !bSlider) return;

  const savedA = parseFloat(localStorage.getItem("libpool-btn-alpha"));
  const savedB = parseInt(localStorage.getItem("libpool-btn-blur"), 10);
  const a = Number.isFinite(savedA) ? savedA : 0.92;
  const b = Number.isFinite(savedB) ? savedB : 18;

  const apply = () => {
    const av = parseFloat(aSlider.value) / 100;
    const bv = parseInt(bSlider.value, 10);
    document.documentElement.style.setProperty("--btn-alpha", av);
    document.documentElement.style.setProperty("--btn-blur", bv + "px");
    aVal.textContent = Math.round(av * 100) + "%";
    bVal.textContent = bv;
    localStorage.setItem("libpool-btn-alpha", av);
    localStorage.setItem("libpool-btn-blur", bv);
  };

  aSlider.value = Math.round(a * 100);
  bSlider.value = b;
  apply();
  aSlider.addEventListener("input", apply);
  bSlider.addEventListener("input", apply);
}

/* ---------------- 壁纸模糊度（设置页滑杆） ----------------
   实时写入 --wall-blur：自定义背景图片的模糊强度（0 = 完全清晰不模糊）。
   默认不再叠加白色半透明遮罩（custom-bg-scrim 背景已改为透明），
   可读性由用户自己选的模糊度来保证。持久化到 localStorage。 */
function setupWallBlur() {
  const slider = $("wallBlurSlider");
  const val = $("wallBlurVal");
  if (!slider || !val) return;

  const saved = parseInt(localStorage.getItem("libpool-wall-blur"), 10);
  const n = Number.isFinite(saved) ? saved : 16;

  const apply = () => {
    const v = parseInt(slider.value, 10);
    document.documentElement.style.setProperty("--wall-blur", v + "px");
    val.textContent = v;
    localStorage.setItem("libpool-wall-blur", v);
  };

  slider.value = n;
  apply();
  slider.addEventListener("input", apply);
}

/* ---------------- 镜像（卡片式单选） ---------------- */
function setupMirror() {
  const listEl = $("mirrorList");
  const customWrap = $("mirrorCustomWrap");
  const custom = $("mirrorCustom");

  const setActive = (btn, persist = false) => {
    listEl.querySelectorAll(".mirror-card").forEach((b) => {
      const on = b === btn;
      b.classList.toggle("active", on);
      b.setAttribute("aria-checked", on);
    });
    customWrap.hidden = !(btn && btn.dataset.url === "custom");
    // 持久化选中态（含自定义地址），重进 WebUI 保持
    if (persist) {
      const url = btn ? btn.dataset.url : "custom";
      localStorage.setItem("libpool-mirror", url);
      if (url === "custom") localStorage.setItem("libpool-mirror-custom", custom.value.trim());
    }
  };

  // 点选镜像卡片：选中即保存
  listEl.addEventListener("click", (e) => {
    const btn = e.target.closest(".mirror-card");
    if (btn) setActive(btn, true);
  });

  // 从已保存配置回填选中态；真机无配置时从 localStorage 回填（预览/浏览器也生效）
  const applyMirrorConfig = (url) => {
    if (!url) {
      const saved = localStorage.getItem("libpool-mirror");
      const savedCustom = localStorage.getItem("libpool-mirror-custom") || "";
      if (saved === "custom") {
        custom.value = savedCustom;
        setActive(listEl.querySelector('[data-url="custom"]'));
      } else if (saved) {
        const b = listEl.querySelector(`[data-url="${saved}"]`);
        if (b) setActive(b);
      }
      return;
    }
    let found = null;
    listEl.querySelectorAll(".mirror-card").forEach((b) => {
      if (b.dataset.url === "custom") return;
      try { if (url.includes(new URL(b.dataset.url).host)) found = b; } catch (e) { /* ignore */ }
    });
    if (found) setActive(found);
    else {
      custom.value = url;
      setActive(listEl.querySelector('[data-url="custom"]'));
    }
  };

  $("btnSaveMirror").addEventListener("click", async () => {
    const active = listEl.querySelector(".mirror-card.active");
    let url = active ? active.dataset.url : "custom";
    if (url === "custom") url = custom.value.trim();
    if (!url || !/^https?:\/\//.test(url)) { showToast("请输入有效的镜像地址", true); return; }
    try {
      const r = await exec(libmanCmd(`config mirror ${url}`));
      if (r.errno !== 0) throw new Error((r.stderr || "").trim());
      state.mirror = url;
      // 持久化完整配置，重进 WebUI 保持
      localStorage.setItem("libpool-mirror", url);
      if (/custom/i.test(active ? active.dataset.url : "") || url === custom.value.trim()) {
        localStorage.setItem("libpool-mirror-custom", url);
      }
      showToast("镜像已保存");
    } catch (e) {
      showToast("保存失败：" + e.message, true);
    }
  });

  return applyMirrorConfig;
}

/* ---------------- 自定义背景图（网址 + 本地相册） ---------------- */
function setupBg() {
  const saved = localStorage.getItem("libpool-bg");
  if (saved) setBg(saved);

  $("btnApplyBg").addEventListener("click", () => {
    const url = $("bgInput").value.trim();
    if (!url) { clearBg(); showToast("已清除背景"); return; }
    if (!/^https?:\/\//.test(url)) { showToast("请输入以 http(s):// 开头的图片网址", true); return; }
    setBg(url);
    localStorage.setItem("libpool-bg", url);
    showToast("背景已应用");
  });

  $("btnPickBg").addEventListener("click", () => $("bgFile").click());
  $("bgFile").addEventListener("change", async (e) => {
    const file = e.target.files && e.target.files[0];
    if (!file) return;
    try {
      const dataUrl = await compressImage(file, 1600, 0.8);
      localStorage.setItem("libpool-bg", dataUrl);
      setBg(dataUrl);
      $("bgInput").value = "";
      showToast("本地图片背景已应用");
    } catch (err) {
      showToast("图片处理失败：" + err.message, true);
    }
    e.target.value = ""; // 允许再次选择同一文件
  });

  $("btnClearBg").addEventListener("click", () => {
    clearBg();
    showToast("已清除背景");
  });
}

/** 压缩本地图片为 JPEG DataURL（限最长边 maxEdge、质量 quality），避免撑爆 localStorage。 */
function compressImage(file, maxEdge, quality) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error("读取文件失败"));
    reader.onload = () => {
      const img = new Image();
      img.onerror = () => reject(new Error("不是有效的图片"));
      img.onload = () => {
        let w = img.width, h = img.height;
        const scale = Math.min(1, maxEdge / Math.max(w, h));
        w = Math.max(1, Math.round(w * scale));
        h = Math.max(1, Math.round(h * scale));
        const canvas = document.createElement("canvas");
        canvas.width = w; canvas.height = h;
        const ctx = canvas.getContext("2d");
        ctx.drawImage(img, 0, 0, w, h);
        resolve(canvas.toDataURL("image/jpeg", quality));
      };
      img.src = reader.result;
    };
    reader.readAsDataURL(file);
  });
}

function setBg(url) {
  const bg = $("customBg");
  bg.style.backgroundImage = `url("${url}")`;
  bg.classList.add("on");
  document.body.classList.add("custom-bg-active");
}

function clearBg() {
  localStorage.removeItem("libpool-bg");
  const bg = $("customBg");
  bg.style.backgroundImage = "";
  bg.classList.remove("on");
  document.body.classList.remove("custom-bg-active");
  $("bgInput").value = "";
}

/* ---------------- 公告（拉取纯文本站点，每天最多一次） ----------------
   网址优先级：模块 .git/url.txt（用户可手动填写）> 内置 ANNOUNCEMENT_URL。
   候选地址逐个尝试、全部失败才跳过；"每天一次"按本地日期判断（修正了 UTC 偏移）。
   公告与云更新共用同一个毛玻璃弹窗：公告先展示，用户关闭后再做云更新检查，
   避免更新弹窗把欢迎公告顶掉（此前"公告不显示"的根因之一）。 */

function localDateKey() {
  const d = new Date();
  const p = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** 按优先级依次尝试候选公告网址，成功返回 {url, text}，全部失败返回 null。 */
async function fetchAnnouncementText() {
  const candidates = [];
  if (!state.inPreview && MOD) {
    try {
      const r = await exec(`cat ${MOD}/.git/url.txt 2>/dev/null`);
      const line = (r.stdout || "").split(/\r?\n/).find((l) => /^https?:\/\/\S+$/.test(l.trim()));
      if (line) candidates.push(line.trim());
    } catch (e) { /* ignore */ }
  }
  if (ANNOUNCEMENT_URL) candidates.push(ANNOUNCEMENT_URL.trim());
  const seen = new Set();
  for (const url of candidates) {
    if (seen.has(url)) continue;
    seen.add(url);
    try {
      const res = await fetch(url, { cache: "no-store" });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const text = (await res.text()).trim();
      if (text) return { url, text };
    } catch (e) {
      logWebui(`公告拉取失败(${url}): ${e.message}`);
      console.warn("公告拉取失败，跳过", url, e);
    }
  }
  return null;
}

/** 把富文本写入公告弹窗（标题 / 徽标 / Markdown 内容）。 */
function renderAnnouncement(title, badge, md) {
  $("announcementTitleText").textContent = title;
  const b = $("announcementBadge");
  if (b) b.textContent = badge;
  $("announcementContent").innerHTML = renderMarkdown(md);
}

/** 每天最多弹一次的欢迎公告。返回是否已展示（供云更新排队判断）。 */
async function loadAnnouncement() {
  const today = localDateKey();
  if (localStorage.getItem("libpool-announce-date") === today) return false; // 今天已看过
  const hit = await fetchAnnouncementText();
  if (!hit) return false; // 未配置或全部候选拉取失败
  renderAnnouncement("公告", "最新", hit.text);
  openAnnouncement();
  localStorage.setItem("libpool-announce-date", today);
  logWebui(`欢迎公告已展示: ${hit.url}`);
  return true;
}

/** 手动重新展示公告（关于页按钮，绕过"每天一次"限制，方便验证）。 */
async function showAnnouncementNow() {
  const hit = await fetchAnnouncementText();
  if (!hit) {
    showToast("公告拉取失败，请检查网络或 .git/url.txt", true);
    return;
  }
  renderAnnouncement("公告", "最新", hit.text);
  openAnnouncement();
  logWebui(`手动重显公告: ${hit.url}`);
}

function openAnnouncement() {
  const el = $("announcement");
  el.hidden = false;
  // 下一帧再加 open，确保 backdrop-filter 从 0 开始渐变
  requestAnimationFrame(() => el.classList.add("open"));
}

const _announceCloseWaiters = [];
/** 等待当前公告弹窗被用户关闭（用于让云更新检查排队在后）。 */
function waitForAnnouncementClose() {
  return new Promise((resolve) => _announceCloseWaiters.push(resolve));
}
function flushAnnouncementCloseWaiters() {
  const ws = _announceCloseWaiters.splice(0);
  ws.forEach((fn) => fn());
}

function closeAnnouncement() {
  const el = $("announcement");
  el.classList.remove("open");
  setTimeout(() => {
    el.hidden = true;
    flushAnnouncementCloseWaiters(); // 公告关闭后再放行云更新检查
  }, 520);
}

function setupAnnouncementUI() {
  $("okAnnouncement").addEventListener("click", closeAnnouncement);
  document.querySelector(".announcement-backdrop").addEventListener("click", closeAnnouncement);
  const rb = $("btnShowAnnouncement");
  if (rb) rb.addEventListener("click", showAnnouncementNow);
  // 弹窗内链接（云更新的下载链接、Markdown 里的链接）点击 → 跳转默认浏览器，不在 WebUI 内导航
  document.querySelector(".announcement-content").addEventListener("click", (e) => {
    const a = e.target.closest("a");
    if (!a) return;
    e.preventDefault();
    openExternal(a.href);
  });
}

/* ---------------- 云更新（版本号对比 + 更新弹窗） ---------------- */

/** 解析云更新配置文件：优先读模块 .git/<file>（用户可手动填写），否则回退内置常量。 */
async function resolveUpdateUrl(file, fallback) {
  if (!state.inPreview && MOD) {
    try {
      const r = await exec(`cat ${MOD}/.git/${file} 2>/dev/null`);
      const txt = r.stdout || "";
      const line = txt.split(/\r?\n/).find((l) => /^https?:\/\/\S+$/.test(l.trim()));
      if (line) return line.trim();
    } catch (e) { /* ignore */ }
  }
  return (fallback || "").trim();
}

/** 读取本地模块版本号（module.prop 的 version=），预览模式回退内置值。 */
async function localVersion() {
  if (!state.inPreview) {
    try {
      const r = await exec(`grep -m1 '^version=' ${MOD}/module.prop`);
      const v = (r.stdout || "").split("=")[1]?.trim();
      if (v) return v;
    } catch (e) { /* ignore */ }
  }
  return "1.0.0";
}

/** 数值化点分版本号并比较：a > b → 1，a < b → -1，相等 → 0。 */
function compareVersions(a, b) {
  const pa = String(a).replace(/^v/i, "").split(".").map((n) => parseInt(n, 10) || 0);
  const pb = String(b).replace(/^v/i, "").split(".").map((n) => parseInt(n, 10) || 0);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const x = pa[i] || 0, y = pb[i] || 0;
    if (x > y) return 1;
    if (x < y) return -1;
  }
  return 0;
}

/** 打开外部浏览器（Android 用 am start 拉起默认浏览器；浏览器预览用 window.open）。 */
async function openExternal(url) {
  if (!state.inPreview) {
    try {
      await exec(`am start -a android.intent.action.VIEW -d '${String(url).replace(/'/g, "''")}'`);
      return;
    } catch (e) { /* 回退 window.open */ }
  }
  window.open(url, "_blank");
}

/** 云端更新检查：拉取版本号 → 与本地对比 → 不一致则弹更新弹窗（外观与公告一致）。 */
async function checkUpdate() {
  const url = await resolveUpdateUrl("update.txt", UPDATE_URL);
  if (!url) return; // 未配置云更新地址
  let cloudV = "";
  try {
    const res = await fetch(url, { cache: "no-store" });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    cloudV = (await res.text()).trim().replace(/^v/i, "");
  } catch (e) {
    console.warn("云更新版本拉取失败，跳过", e);
    return;
  }
  const localV = await localVersion();
  if (!cloudV || cloudV === localV) return;
  // 同一云端版本只提醒一次（避免每次打开都打扰）；出现更新的云端版本时重新提醒
  if (localStorage.getItem("libpool-update-seen") === cloudV) return;
  localStorage.setItem("libpool-update-seen", cloudV);

  const dl = (await resolveUpdateUrl("download.txt", UPDATE_DOWNLOAD_URL)) || "";
  const newer = compareVersions(cloudV, localV) > 0;
  if (newer) {
    const target = dl || UPDATE_DOWNLOAD_URL;
    const label = dl ? `前往下载 v${cloudV} →` : `GitHub Releases →`;
    renderAnnouncement("发现新版本", "更新",
      `检测到云端新版本 **v${cloudV}**（当前本地 v${localV}）。\n\n[${label}](${target})`);
  } else {
    renderAnnouncement("发现新版本", "更新",
      `云端版本 v${cloudV} 与本地 v${localV} 不一致（云端较旧）。`);
  }
  openAnnouncement();
}

/* ---------------- 搜索（防抖 + 重建跳过动画） ---------------- */
function setupSearch() {
  let timer = null;
  $("searchInput").addEventListener("input", (e) => {
    state.query = e.target.value;
    clearTimeout(timer);
    timer = setTimeout(() => renderStoreGrid(false), 150);
  });
}

/* ---------------- 涟漪（触摸优先核心动效） ----------------
   事件委托：动态渲染的元素也会生效。
   快速连点时，新涟漪从新触点叠加扩散，旧涟漪被 overflow 裁剪平滑结束（并行动画打断）。 */
function setupRipple() {
  document.addEventListener("pointerdown", (e) => {
    const host = e.target.closest(".ripple-host");
    if (!host) return;
    const rect = host.getBoundingClientRect();
    const d = Math.max(rect.width, rect.height) * 1.2;
    const span = document.createElement("span");
    span.className = "ripple";
    span.style.width = span.style.height = d + "px";
    span.style.left = e.clientX - rect.left - d / 2 + "px";
    span.style.top = e.clientY - rect.top - d / 2 + "px";
    host.appendChild(span);
    span.addEventListener("animationend", () => span.remove());
  }, { passive: true });
}

/* ---------------- 滚动时上下边框模糊过渡 ---------------- */
function setupScrollBlur() {
  const onScroll = () => {
    document.body.classList.toggle("scrolled", window.scrollY > 4);
  };
  window.addEventListener("scroll", onScroll, { passive: true });
  onScroll();
}

/* ---------------- 返回顶部（毛玻璃悬浮键） ----------------
   滚动进度超 15% 时从底部丝滑弹出，点击平滑回到顶部；
   透明度/模糊由 --btn-alpha/--btn-blur 控制（绑定设置里的"按钮"滑杆）。 */
function setupBackToTop() {
  const btn = $("backTop");
  if (!btn) return;
  const tabsEl = document.querySelector(".tabs");
  const onScroll = () => {
    const doc = document.documentElement;
    const progress = window.scrollY / Math.max(1, doc.scrollHeight - window.innerHeight);
    // 扩展库页（store）滚动到 10% 就出现，其它页面 15%
    const isStore = tabsEl && tabsEl.dataset.active === "store";
    const threshold = isStore ? 0.10 : 0.15;
    btn.classList.toggle("show", progress > threshold);
  };
  window.addEventListener("scroll", onScroll, { passive: true });
  // 切换 Tab（已滚动）时立即重算，无需再滚动
  if (tabsEl) {
    new MutationObserver(onScroll).observe(tabsEl, { attributes: true, attributeFilter: ["data-active"] });
  }
  onScroll();
  btn.addEventListener("click", () => window.scrollTo({ top: 0, behavior: "smooth" }));
}

/* ---------------- 滚动入场（下滑时组件逐个滑入） ----------------
   用 IntersectionObserver（不监听 scroll，避免逐帧卡顿），元素进入视口约 12% 时加 .in；
   既保证"滑动中触发"（不必等停稳），又避免 threshold 0 导致动画在用户注意到前就完成；
   先快后慢曲线 + 位移 28px + 时长 0.55s + 按 --i 小错峰，由 CSS .reveal/.reveal.in 实现。
   display:none 面板内的元素 IO 不会自动触发，故面板切换时调用 revealRecheck 强制重检。 */
let revealRecheck = null;

function setupScrollReveal() {
  const io = new IntersectionObserver((entries) => {
    for (const ent of entries) {
      if (ent.isIntersecting) {
        ent.target.classList.add("in");
        io.unobserve(ent.target);
      }
    }
  }, { threshold: 0.12, rootMargin: "0px 0px 0px 0px" });

  const observeReveals = () => {
    document.querySelectorAll(".reveal:not(.in)").forEach((el) => io.observe(el));
  };
  observeReveals();

  // 面板切换 / 动态重建后强制重检（先 unobserve 再 observe，触发一次新回调）
  revealRecheck = () => {
    document.querySelectorAll(".reveal").forEach((el) => io.unobserve(el));
    observeReveals();
  };

  // 动态重建列表（搜索/切分类/刷新）后重新观察新卡片
  const mo = new MutationObserver(observeReveals);
  mo.observe($("storeGrid"), { childList: true });
  mo.observe($("mountedList"), { childList: true });
}

/* ---------------- 重置 ---------------- */
function setupReset() {
  $("btnReset").addEventListener("click", async () => {
    if (state.inPreview) return showToast("预览模式下不执行操作");
    if (!confirm("确定要卸载全部库并重置吗？此操作会释放所有已挂载的库。")) return;
    try {
      const r = await exec(libmanCmd("reset"));
      if (r.errno !== 0) throw new Error((r.stderr || "").trim());
      showToast("已重置");
      await refresh();
    } catch (e) {
      showToast("重置失败：" + e.message, true);
    }
  });
}

/* ---------------- Overlay / Toast ---------------- */
function showOverlay(title, sub) {
  $("overlayTitle").textContent = title;
  $("overlaySub").textContent = sub;
  // 真实百分比未知时用不确定光条，不再造假百分比
  $("progressFill").classList.add("indeterminate");
  $("progressFill").style.width = "";
  $("overlay").hidden = false;
}
function hideOverlay() {
  $("overlay").hidden = true;
}

let toastTimer = null;
let toastHideTimer = null;
function showToast(msg, isError = false) {
  const el = $("toast");
  el.textContent = msg;
  el.classList.toggle("error", isError);  // 亮暗双色 + 毛玻璃 G2 由 CSS 变量控制
  clearTimeout(toastHideTimer);
  el.hidden = false;
  // 下一帧再加 .visible，让 backdrop-filter/transform 从 0 平滑进场（先快后慢）
  requestAnimationFrame(() => el.classList.add("visible"));
  clearTimeout(toastTimer);
  // 退场：先移除 .visible 走退场过渡（滑落 + 淡出），过渡结束再隐藏
  toastTimer = setTimeout(() => {
    el.classList.remove("visible");
    toastHideTimer = setTimeout(() => { el.hidden = true; }, 320);
  }, 2200);
}

/* ---------------- 动效助手 ---------------- */

/* 统计数字 count-up：easeOutCubic 先快后慢，纯视觉、不依赖鼠标 */
function animateNumber(el, target) {
  const cur = parseFloat(el.textContent) || 0;
  if (cur === target) return;
  const dur = 600;
  const start = performance.now();
  const step = (now) => {
    const p = Math.min(1, (now - start) / dur);
    const eased = 1 - Math.pow(1 - p, 3);
    el.textContent = Math.round(cur + (target - cur) * eased);
    if (p < 1) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}

/* 安装成功脉冲：给对应卡片套一圈光晕扩散，由 CSS .just-installed 实现 */
function pulseCard(id) {
  const btn = document.querySelector(`.store-card [data-action][data-id="${id}"]`);
  const card = btn ? btn.closest(".store-card") : null;
  if (!card) return;
  card.classList.add("just-installed");
  setTimeout(() => card.classList.remove("just-installed"), 950);
}

/* ---------------- 工具 ---------------- */
function escapeHtml(s) {
  return String(s == null ? "" : s)
    .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

/* ---------------- 极简 Markdown 渲染（公告 / 云更新弹窗用） ----------------
   先整体转义 HTML 再按行/内联转成富文本，保证注入安全。
   支持：# 标题、**粗体**、*斜体*、`行内代码`、```围栏代码块```、
   [链接](https://…)、- 无序列表、1. 有序列表、> 引用、--- 分隔线、段落。
   链接统一加 update-link 类（蓝色下划线 + 点击跳默认浏览器）。 */
function renderMarkdown(src) {
  const esc = escapeHtml(src || "");
  const inline = (s) =>
    s
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
      .replace(/\*([^*]+)\*/g, "<em>$1</em>")
      .replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '<a class="update-link" href="$2">$1</a>');

  const lines = esc.split(/\r?\n/);
  let html = "";
  let listType = null;   // "ul" | "ol"
  let inCode = false;
  let codeBuf = [];

  for (const raw of lines) {
    if (/^```/.test(raw)) {   // 围栏代码块
      if (!inCode) { inCode = true; codeBuf = []; }
      else { html += "<pre><code>" + codeBuf.join("\n") + "</code></pre>"; inCode = false; }
      continue;
    }
    if (inCode) { codeBuf.push(raw); continue; }

    if (!raw.trim()) {         // 空行：结束列表
      if (listType) { html += "</" + listType + ">"; listType = null; }
      continue;
    }

    const h = /^(#{1,3})\s+(.*)$/.exec(raw);
    if (h) {
      if (listType) { html += "</" + listType + ">"; listType = null; }
      html += "<h" + h[1].length + ">" + inline(h[2]) + "</h" + h[1].length + ">";
      continue;
    }
    if (/^-{3,}$/.test(raw.trim())) {   // 分隔线
      if (listType) { html += "</" + listType + ">"; listType = null; }
      html += "<hr>";
      continue;
    }
    if (/^&gt;\s?/.test(raw)) {         // 引用（转义后是 &gt;）
      if (listType) { html += "</" + listType + ">"; listType = null; }
      html += "<blockquote>" + inline(raw.replace(/^&gt;\s?/, "")) + "</blockquote>";
      continue;
    }
    const ul = /^[-*]\s+(.*)$/.exec(raw);
    if (ul) {
      if (listType !== "ul") { if (listType) html += "</" + listType + ">"; html += "<ul>"; listType = "ul"; }
      html += "<li>" + inline(ul[1]) + "</li>";
      continue;
    }
    const ol = /^\d+[.)]\s+(.*)$/.exec(raw);
    if (ol) {
      if (listType !== "ol") { if (listType) html += "</" + listType + ">"; html += "<ol>"; listType = "ol"; }
      html += "<li>" + inline(ol[1]) + "</li>";
      continue;
    }
    if (listType) { html += "</" + listType + ">"; listType = null; }
    html += "<p>" + inline(raw) + "</p>";
  }
  if (inCode) html += "<pre><code>" + codeBuf.join("\n") + "</code></pre>";
  if (listType) html += "</" + listType + ">";
  return html;
}

/* ---------------- 模糊预热（秒加载） ----------------
   切换页面/面板时毛玻璃"晚 0.x 秒才出现"的根因：backdrop-filter 的 GPU
   着色器是首次渲染才编译。启动时先用 1px 不可见层渲染一次标准滤镜栈，
   提前把着色器编译好；之后所有毛玻璃首帧即清晰，人眼感知不到加载过程。 */
function warmupBlur() {
  const w = $("blurWarmup");
  if (!w) return;
  void w.offsetHeight;   // 强制同步布局，确保首帧真实绘制触发着色器编译
  requestAnimationFrame(() => requestAnimationFrame(() => {
    w.classList.add("ready");   // 编译完成后移除预热层，释放资源
  }));
}

/* ---------------- 高配机型 GPU 加速（设置页可开关） ----------------
   骁龙8Gen2 / 天玑9300 及以上通常 ≥8 核，默认自动开启；
   给 <html> 加 .gpu 后，CSS 给所有毛玻璃面建独立 GPU 合成层
   （will-change: backdrop-filter），模糊由 GPU 独立渲染、进一步加速。
   低配机型默认关闭，避免内存/开销反噬。用户可在设置里手动切换并记住选择。 */
function applyGpu(on) {
  document.documentElement.classList.toggle("gpu", !!on);
  const sw = $("gpuSwitch");
  if (sw) { sw.classList.toggle("on", !!on); sw.setAttribute("aria-checked", on ? "true" : "false"); }
}

function setupGpu() {
  const sw = $("gpuSwitch");
  const saved = localStorage.getItem("libpool-gpu");
  const auto = (navigator.hardwareConcurrency || 0) >= 8;
  applyGpu(saved === null ? auto : saved === "1");
  if (sw) {
    sw.addEventListener("click", () => {
      const next = !document.documentElement.classList.contains("gpu");
      applyGpu(next);
      localStorage.setItem("libpool-gpu", next ? "1" : "0");
      showToast(next ? "已开启 GPU 加速" : "已关闭 GPU 加速");
    });
  }
}

/* ---------------- 日志系统（模块自诊断） ----------------
   日志文件：
     install.log    安装脚本（customize.sh）
     service.log    开机服务（service.sh）
     logs/webui.log  WebUI 运行诊断（启动/工具预检/操作/公告）
     logs/libman.log libman 工具运行日志（命令调用与错误）
     logs/diagnose.log  实时运行诊断快照
   写入用 exec 追加（best-effort，失败不影响界面）；「设置 → 查看日志」可读全部日志。
   日志查看器置顶展示实时诊断（架构 / tools 目录权限 / 各工具是否可执行），
   直接定位"管理工具未就绪"的根因（缺失？权限？反斜杠遗留？）。 */

/** 安全地把一行文本追加到模块内的日志文件（处理单引号与换行）。 */
async function appendToLog(file, text) {
  if (state.inPreview || !MOD) return;
  try {
    const safe = String(text).replace(/'/g, "'\\''");
    await exec(`mkdir -p ${MOD}/logs; printf '%s\n' '${safe}' >> ${MOD}/${file}`);
  } catch (e) { /* 日志失败忽略 */ }
}

async function logWebui(msg) {
  const t = new Date().toLocaleString("zh-CN", { hour12: false });
  await appendToLog("logs/webui.log", `[${t}] ${msg}`);
}

/** 实时运行诊断：探测架构、tools 目录、各候选工具可执行性、libman 版本。
    结果写入 logs/diagnose.log 并返回文本（日志查看器置顶显示）。 */
async function runDiagnostics() {
  if (state.inPreview || !MOD) return "（预览模式，无设备诊断）\n";
  try {
    const r = await exec(
      `chmod 755 ${MOD}/tools 2>/dev/null; chmod 755 ${MOD}/tools/* 2>/dev/null; ` +  // 打开日志即顺带修复 +x
      `echo "架构: $(uname -m)"; echo; ` +
      `echo '--- tools 目录 ---'; ls -la ${MOD}/tools 2>&1; echo; ` +
      `echo '--- 工具可执行性 ---'; ` +
      `test -x ${MOD}/tools/libman && echo 'libman      可执行' || echo 'libman      缺失或不可执行'; ` +
      `test -x ${MOD}/tools/libman-arm && echo 'libman-arm  可执行' || echo 'libman-arm  缺失或不可执行'; ` +
      `test -x ${MOD}/tools/libman.sh && echo 'libman.sh   可执行' || echo 'libman.sh   缺失或不可执行'; echo; ` +
      `echo '--- libman 版本 ---'; ` +
      `{ test -x ${MOD}/tools/libman && ${MOD}/tools/libman version 2>&1; } || echo 'libman 无法运行'`
    );
    const diag = `===== 运行诊断 =====\n${(r.stdout || "").trim()}\n\n`;
    await appendToLog("logs/diagnose.log", diag);
    return diag;
  } catch (e) {
    return "";
  }
}

async function openLogViewer() {
  const files = ["install.log", "service.log", "logs/webui.log", "logs/libman.log", "logs/diagnose.log"];
  let out = await runDiagnostics(); // 置顶实时诊断，一眼定位工具未就绪原因
  for (const f of files) {
    try {
      const r = await exec(`cat ${MOD}/${f} 2>/dev/null`);
      const content = (r.stdout || "").replace(/\s+$/, "");
      if (content) out += `===== ${f} =====\n${content}\n\n`;
    } catch (e) { /* ignore */ }
  }
  $("logView").textContent = out.trim() || "暂无日志，请先安装/重启模块生成。";
  const el = $("logModal");
  el.hidden = false;
  requestAnimationFrame(() => el.classList.add("open"));
}
function closeLogViewer() {
  const el = $("logModal");
  el.classList.remove("open");
  setTimeout(() => { el.hidden = true; }, 320);
}
function setupLogUI() {
  $("btnLogView").addEventListener("click", openLogViewer);
  $("btnLogClose").addEventListener("click", closeLogViewer);
  $("btnLogCopy").addEventListener("click", async () => {
    const text = $("logView").textContent;
    try { await navigator.clipboard.writeText(text); showToast("日志已复制"); }
    catch (e) { showToast("复制失败，请手动长按选择"); }
  });
  // 点击背景关闭
  document.querySelector("#logModal .announcement-backdrop").addEventListener("click", closeLogViewer);
}

/* ---------------- 启动 ---------------- */
(async function init() {
  try { enableEdgeToEdge(true); } catch (e) { /* noop */ }
  try { fullScreen(true); } catch (e) { /* noop */ }

  setupGpu();   // 高配机型 GPU 加速（默认自动，设置页可开关）
  warmupBlur();   // 先预热毛玻璃着色器，切换页面时秒加载
  setupTabs();
  setupTheme();
  setupGlass();
  setupRadius();
  setupBtnGlass();
  setupSearch();
  setupRipple();
  setupScrollBlur();
  setupBackToTop();
  setupReset();
  setupBg();
  setupWallBlur();
  setupAnnouncementUI();
  setupLogUI();
  $("btnRefresh").addEventListener("click", refresh);
  $("btnCancelDownload").addEventListener("click", cancelDownload);

  await detectEnvironment();
  logWebui(`启动：模块=${MOD} 预览=${state.inPreview} 原生工具=${useNative}`);
  await loadRepos();
  await loadLibs();
  await loadMirror();

  // 镜像回填（需在 loadMirror 之后）
  setupMirror()(state.mirror);

  renderMounted();
  renderStore();
  setupScrollReveal();   // 首屏卡片渲染后启用滚动入场
  renderStatus();
  $("aboutVer").textContent = await localVersion();

  if (!state.inPreview) {
    try {
      const out = await libmanExec("status");
      const st = JSON.parse(out);
      $("aboutArch").textContent = st.arch || "aarch64";
    } catch (e) { /* ignore */ }
  }

  // 欢迎公告优先展示（每天一次）；用户关闭后放行云更新检查，
  // 避免更新弹窗把欢迎公告顶掉（此前"公告不显示"的根因之一）
  const annShown = await loadAnnouncement();
  if (annShown) {
    waitForAnnouncementClose().then(() => checkUpdate());
  } else {
    checkUpdate();
  }
})();
