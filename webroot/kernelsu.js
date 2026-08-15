/**
 * kernelsu.js — KernelSU WebUI 官方 JS 库（v3.0.2，0 依赖）内联版
 * LibPool 适配：Copyright (C) 2026 MINO · Himer (MineACE)
 * SPDX-License-Identifier: Apache-2.0
 * 通信对象：KernelSU 管理器注入的全局 `ksu`。
 * 若在非 KernelSU 环境（浏览器预览）运行，自动降级为 mock，便于开发调试。
 */
(function () {
  const hasBridge = typeof window !== "undefined" && typeof window.ksu === "object" && window.ksu !== null;

  // ---------------- mock 实现（浏览器预览用） ----------------
  // 注意：实际 ksu 桥接里 exec/spawn 的回调参数是「回调名(字符串)」，
  // 需要在 window 上取对应函数。mock 必须模拟这一契约。
  const mock = {
    exec(command, options, cb) {
      console.log("[mock exec]", command);
      const stdout = "[]";
      setTimeout(() => {
        const fn = typeof cb === "function" ? cb : window[cb];
        if (typeof fn === "function") fn(0, stdout, "");
      }, 120);
    },
    spawn(command, args, options, cb) {
      console.log("[mock spawn]", command, args);
      setTimeout(() => {
        const child = typeof cb === "object" && cb !== null ? cb : window[cb];
        if (child && typeof child.emit === "function") child.emit("exit", 0);
      }, 120);
    },
    toast(message) {
      console.log("[mock toast]", message);
    },
    fullScreen() {},
    enableEdgeToEdge() {},
    moduleInfo() {
      return JSON.stringify({ id: "libpool", name: "LibPool" });
    },
    listPackages() {
      return "[]";
    },
    getPackagesInfo() {
      return "[]";
    },
    exit() {},
  };

  const isMock = !hasBridge;
  const bridge = hasBridge ? window.ksu : mock;
  if (!hasBridge) {
    console.warn("[kernelsu] 未检测到 ksu 桥接，使用 mock 模式（仅用于浏览器预览）");
  }

  let callbackCounter = 0;
  function getUniqueCallbackName(prefix) {
    return `${prefix}_callback_${Date.now()}_${callbackCounter++}`;
  }

  function cleanup(name) {
    try { delete window[name]; } catch (e) { /* noop */ }
  }

  // ---------------- 官方 API 实现 ----------------
  const api = {
    exec(command, options) {
      if (typeof options === "undefined") options = {};
      return new Promise((resolve, reject) => {
        const callbackFuncName = getUniqueCallbackName("exec");
        window[callbackFuncName] = (errno, stdout, stderr) => {
          resolve({ errno, stdout, stderr });
          cleanup(callbackFuncName);
        };
        try {
          bridge.exec(command, JSON.stringify(options), callbackFuncName);
        } catch (error) {
          reject(error);
          cleanup(callbackFuncName);
        }
      });
    },

    spawn(command, args, options) {
      if (typeof args === "undefined") args = [];
      else if (!(args instanceof Array)) options = args;
      if (typeof options === "undefined") options = {};

      const listeners = { stdout: [], stderr: [], exit: [], error: [] };
      const child = {
        stdin: { on() {} },
        stdout: { on(ev, fn) { if (ev === "data") listeners.stdout.push(fn); } },
        stderr: { on(ev, fn) { if (ev === "data") listeners.stderr.push(fn); } },
        on(ev, fn) { if (listeners[ev]) listeners[ev].push(fn); },
        emit(ev, ...args) { (listeners[ev] || []).forEach((fn) => fn(...args)); },
      };
      const childCallbackName = getUniqueCallbackName("spawn");
      window[childCallbackName] = child;
      child.on("exit", () => cleanup(childCallbackName));
      try {
        bridge.spawn(command, JSON.stringify(args), JSON.stringify(options), childCallbackName);
      } catch (error) {
        child.emit("error", error);
        cleanup(childCallbackName);
      }
      return child;
    },

    fullScreen(isFullScreen) { bridge.fullScreen(isFullScreen); },
    enableEdgeToEdge(enable) { bridge.enableEdgeToEdge(enable); },
    toast(message) { try { bridge.toast(message); } catch (e) { console.warn(e); } },
    moduleInfo() {
      try {
        const raw = bridge.moduleInfo();
        return typeof raw === "string" ? JSON.parse(raw) : raw;
      } catch (e) {
        return { id: "libpool", name: "LibPool" };
      }
    },
    listPackages(type) {
      try { return JSON.parse(bridge.listPackages(type)); } catch (e) { return []; }
    },
    getPackagesInfo(packages) {
      try {
        if (typeof packages !== "string") packages = JSON.stringify(packages);
        return JSON.parse(bridge.getPackagesInfo(packages));
      } catch (e) { return []; }
    },
    exit() { bridge.exit(); },
  };

  // 暴露为 ES module + 全局（双保险）
  api.isMock = isMock;
  window.__kernelsu = api;
})();

export const exec = window.__kernelsu.exec;
export const spawn = window.__kernelsu.spawn;
export const fullScreen = window.__kernelsu.fullScreen;
export const enableEdgeToEdge = window.__kernelsu.enableEdgeToEdge;
export const toast = window.__kernelsu.toast;
export const moduleInfo = window.__kernelsu.moduleInfo;
export const listPackages = window.__kernelsu.listPackages;
export const getPackagesInfo = window.__kernelsu.getPackagesInfo;
export const exit = window.__kernelsu.exit;
