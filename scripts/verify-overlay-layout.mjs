import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const overlayPath = fileURLToPath(new URL("../src-tauri/src/overlay.rs", import.meta.url));
const source = await readFile(overlayPath, "utf8");

assert.match(
  source,
  /html\{height:100%!important;padding-top:40px!important;box-sizing:border-box!important;overflow:hidden!important\}/,
  "标题栏必须包含在根节点高度内，不能制造整页纵向滚动",
);
assert.match(
  source,
  /body,#root\{height:100%!important;min-height:0!important;overflow:hidden!important\}/,
  "只有 Harness 的内部内容区可以滚动，页面根节点必须固定",
);
assert.doesNotMatch(
  source,
  /body\{min-height:calc\(100vh - 40px\)!important\}/,
  "不能用 viewport 最小高度叠加标题栏内边距",
);

console.log("覆盖层根布局滚动限制验证通过");