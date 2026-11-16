# 更新包签名

发布构建可为每个 `AALC.nsh_<version>.7z` 生成同名的
`*.manifest.json` 与 `*.manifest.sig`。清单绑定版本、文件名、大小和
SHA-256，签名使用 Ed25519；更新器在解压和终止旧进程前完成校验。

GitHub Actions 使用受保护的 `AALC_UPDATE_SIGNING_KEY` secret（PEM、
base64 或 64 位十六进制私钥）签名，并使用
`AALC_UPDATE_PUBLIC_KEYS`（JSON `keyId -> base64/hex/PEM` 映射）把公钥
嵌入发布包；更新器只信任当前安装目录中已经存在的公钥文件。私钥绝不写入构建产物。
轮换密钥时先把新 key 加入映射，再
切换 `keyId`，待旧版本淘汰后再移除旧 key。

本地可用以下命令验证流程：

```powershell
$env:AALC_UPDATE_SIGNING_KEY = "<ed25519-private-key>"
uv run python scripts/sign_update_manifest.py `
  --archive dist/AALC.nsh_v1.2.3.7z `
  --manifest dist/AALC.nsh_v1.2.3.manifest.json `
  --signature dist/AALC.nsh_v1.2.3.manifest.sig `
  --version v1.2.3
```

更新器默认强制校验：缺少签名清单或签名不匹配时会拒绝安装（fail
closed）。仅当确实要对接无法提供签名的旧发布源时，显式设置
`AALC_UPDATE_REQUIRE_SIGNATURE=0`（`false`/`no`/`off` 同样视为关闭）
才会退回兼容模式。新发布流程在配置 signing secret 后会自动上传签名元数据；
`module/update/checker.py` 启动 `AALC Updater.exe` 时也会显式传递
`AALC_UPDATE_REQUIRE_SIGNATURE=1`，避免环境继承导致降级。

## 原生更新器

`AALC Updater.exe` 默认由 `native/updater/` 的 Rust crate 构建。它在解压和终止
进程之前，按磁盘上的原始 manifest 字节完成 Ed25519 校验，并核对归档文件名、大小和
SHA-256；随后定位 `AALC.nsh/` 解压根目录（不再按压缩包文件名猜测），停止安装目录内
的进程并覆盖安装。旧版 Python 更新器仍可通过 `scripts/build.py --legacy-updater`
构建，作为一版回滚路径保留。

Rust 更新器接受 raw 32 字节、base64、hex 和 PKCS#8 PEM 公钥；签名接受 64 字节原始
文件、base64 或 hex。`scripts/smoke_native_updater.py` 使用真实签名包覆盖全量安装、
增量安装、篡改拒绝、旧 Runner 布局迁移和更新器交接场景。

## 自动安装

设置页「更新与源配置」里的「安装更新」通过 `app.installUpdate` 让 sidecar：

1. 用 `UpdateThread` 检查最新版本；
2. 读取最新 GitHub Release 的 `.7z`、`.manifest.json` 和 `.manifest.sig`；
3. 下载到 `update_temp`；
4. 启动 `AALC Updater.exe`，由它验签、停止应用、覆盖安装并重启。

自动安装目前只支持 GitHub 源；Mirror 酱源仍可用于检查版本和手动下载。V0.0.7 及更早
的安装在首次升级到签名版本时还没有 `update_public_keys.json`，需要手动安装一次
V0.0.8 或更新版本；从该版本起 `update_public_keys.json` 随包提供，后续可以在应用内
直接更新。
