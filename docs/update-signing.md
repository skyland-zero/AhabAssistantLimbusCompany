# 更新包签名

发布构建可为每个 `AALC_<version>.7z` 生成同名的
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
  --archive dist/AALC_v1.2.3.7z `
  --manifest dist/AALC_v1.2.3.manifest.json `
  --signature dist/AALC_v1.2.3.manifest.sig `
  --version v1.2.3
```

兼容旧发布源时，缺少签名清单仍可下载；设置
`AALC_UPDATE_REQUIRE_SIGNATURE=1` 可切换为强制模式。新发布流程在
配置 signing secret 后会自动上传签名元数据。
