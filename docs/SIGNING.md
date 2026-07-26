# Windows 代码签名

本文记录 ClipMaster 安装包的签名方案、供应商取舍与接入步骤。当前发布物**尚未签名**，Windows SmartScreen 会提示"未知发布者"；仓库已内置可选签名流水线，配好凭据即自动生效。

## 背景约束

- 2023 年起 CA/B Forum 强制所有新发代码签名证书必须绑定硬件令牌或云签名服务，不再签发可随意复制的 `.p12` 软证书。
- SmartScreen 信誉按"签名身份"逐步累积：换新证书 = 信誉从零开始，因此供应商宜一次选定、长期使用。

## 供应商对比（2026-07 复核）

| 方案 | 费用 | 适用性 | 结论 |
| --- | --- | --- | --- |
| **SignPath Foundation** | 免费（开源项目） | 按开源仓库审核而非个人身份，无地域限制；证书由 Sectigo 签发，发布者名称为项目名 | **首选**。本仓库（公开 + MIT + 有 CI）符合申请条件 |
| Certum 开源代码签名 | 约 €69/年 | 面向开源作者，SimplySign 云签或 USB 卡，中国大陆可申请 | 后备选项：SignPath 审核不通过时使用 |
| Azure Artifact Signing（原 Trusted Signing） | $9.99/月 | 仅限美国 / 加拿大 / 欧盟 / 英国的企业或个体经营者 | 当前发布者不在适用地区，暂不可用 |
| 传统 OV/EV 证书（DigiCert 等） | $200–600/年 + 硬件令牌 | 通用 | 对个人开源项目成本过高，不采用 |

## 推荐路径：SignPath Foundation

1. 在 <https://signpath.org/apply> 以本仓库地址提交开源项目申请（要求：OSI 许可证、公开仓库、构建来自 CI）。
2. 获批后在 SignPath 控制台创建：Organization、Project（关联本 GitHub 仓库）、Signing Policy（`release-signing`）与 Artifact Configuration（NSIS `*.exe` + MSI `*.msi`，深度签名内部 `clipmaster.exe`）。
3. 在本仓库 GitHub Settings → Secrets and variables → Actions 配置：

   | 名称 | 类型 | 含义 |
   | --- | --- | --- |
   | `SIGNPATH_API_TOKEN` | Secret | SignPath CI 用户的 API Token |
   | `SIGNPATH_ORGANIZATION_ID` | Variable | SignPath 组织 ID |
   | `SIGNPATH_PROJECT_SLUG` | Variable | SignPath 项目 slug |
   | `SIGNPATH_POLICY_SLUG` | Variable | 签名策略 slug（如 `release-signing`） |

4. 重新触发 `Release Build` 工作流：`sign-installers` job 会把未签名产物提交 SignPath 签名，回传后重新生成 `SHA256SUMS.txt` 并上传 `clipmaster-windows-installers-signed` 产物。发布 Release 时改用已签名产物。

凭据未配置时，`sign-installers` job 自动跳过，发布流程与现状完全一致。

## 后备路径：Certum + Tauri signCommand

Certum SimplySign 云签场景下，本地或 CI 通过 Tauri v2 的 `bundle.windows.signCommand` 挂接 `signtool.exe`/SimplySign Desktop 完成签名（示例，配置在 `tauri.conf.json`）：

```json
{
  "bundle": {
    "windows": {
      "signCommand": "signtool sign /n \"<证书主体名>\" /tr http://time.certum.pl /td sha256 /fd sha256 %1"
    }
  }
}
```

该方式无需改动 CI 结构，但签名密钥在本地/证书云端，需自行保管登录凭据。仓库默认不启用。

## 验证签名

```powershell
Get-AuthenticodeSignature .\ClipMaster_x64-setup.exe | Format-List Status, SignerCertificate
signtool verify /pa /v ClipMaster_x64-setup.exe
```

## 现状与信息来源

- 现状：未签名；SmartScreen 提示属预期行为，用户应通过 Release 页 `SHA256SUMS.txt` 校验完整性。
- 参考：[Microsoft 代码签名选项](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) · [Azure Artifact Signing](https://azure.microsoft.com/en-us/products/artifact-signing) · [SignPath Foundation](https://signpath.org/) · [Certum Open Source](https://shop.certum.eu/open-source-code-signing.html)
