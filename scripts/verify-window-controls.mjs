import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const overlayPath = fileURLToPath(new URL("../src-tauri/src/overlay.rs", import.meta.url));
const libPath = fileURLToPath(new URL("../src-tauri/src/lib.rs", import.meta.url));
const source = await readFile(overlayPath, "utf8");
const appSource = await readFile(libPath, "utf8");

assert.match(
  source,
  /const windowActions = \{ minimize: 'minimize', maximize: 'toggle_maximize', close: 'close' \};/,
  "覆盖层必须把最大化按钮映射为后端支持的 toggle_maximize 操作",
);
assert.match(
  source,
  /function windowCommand\(action\) \{\s+if \(!invoke\) return Promise\.reject\(new Error\('窗口控制暂不可用'\)\);\s+return invoke\('window_action', \{ action \}\);\s+\}/,
  "IPC 桥接不可用时，窗口按钮必须报错而不能静默成功",
);
assert.match(
  appSource,
  /payload\.event\(\) == PageLoadEvent::Finished[\s\S]*payload\.url\(\)\.host_str\(\) == Some\("127\.0\.0\.1"\)[\s\S]*payload\.url\(\)\.port\(\) == Some\(3080\)/,
  "Harness 页面完成导航后必须恢复顶栏覆盖层",
);
assert.match(
  appSource,
  /webview\.eval\(overlay_script\(\)\);/,
  "Harness 页面完成导航后必须重新注入顶栏覆盖层",
);

const script = source.match(/r#"([\s\S]*)"#,\s*\n\s*\)/)?.[1];
assert.ok(script, "必须能提取出注入 Harness 页面的覆盖层脚本");
new Function(script);

console.log("窗口控制映射、桥接失败行为与脚本语法验证通过");