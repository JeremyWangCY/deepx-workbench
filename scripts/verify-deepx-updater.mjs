import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const commandsPath = fileURLToPath(new URL("../src-tauri/src/commands.rs", import.meta.url));
const source = await readFile(commandsPath, "utf8");

assert.match(
  source,
  /fn download_deepx_installer\(asset_url: &str, installer: &std::path::Path\)[\s\S]*?Command::new\("powershell\.exe"\)[\s\S]*?Invoke-WebRequest -UseBasicParsing -Uri \$env:DEEPX_UPDATE_URL -OutFile \$env:DEEPX_UPDATE_PATH/,
  "DeepX 更新必须使用 Windows 系统下载器，避免 reqwest 在 GitHub 发布资产重定向上失败",
);
assert.match(
  source,
  /download_deepx_installer\(asset_url, &installer\)\?;[\s\S]*?signature != \*b"MZ"/,
  "下载完成后必须验证安装包签名",
);

console.log("DeepX Windows 更新下载链路验证通过");