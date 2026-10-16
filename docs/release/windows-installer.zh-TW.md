# Windows 安裝程式與簽章

[English](windows-installer.md) · [開發者文件](../development/README.zh-TW.md)

## 支援的發行形式

Beatforge 目前以 Windows 10/11 x64 為目標。採目前使用者範圍的 NSIS installer，
安裝不要求管理員權限。程式裝在 `%LOCALAPPDATA%\Beatforge`，並在開始功能表的
Beatforge 資料夾建立捷徑。舊版不能無提示覆蓋新版。

缺少 WebView2 時 installer 會下載 Microsoft bootstrapper；因此首次安裝可能
需要網路。Visual C++ runtime DLL 已內含。Windows 7、ARM64、32-bit 不支援。
NVIDIA CUDA Toolkit、Git、Python、FFmpeg 與模型權重都不是 installer 的
預先依賴；首次設定會下載固定版本的 Python／AI 環境。建議 NVIDIA GPU，
但環境檢查也可選 CPU 模式。硬體矩陣見
[使用者摘要](../user/requirements.zh-TW.md)。

## 使用者資料界線

程式檔案在 `%LOCALAPPDATA%\Beatforge`；設定、任務歷史、log、生成輸出、
runtime 和模型則位於 Tauri `app_data_dir()` 對應的
`%APPDATA%\com.beatforge.desktop`。重新安裝只能更換程式檔，不應刪除
app-data。一般解除安裝只移除程式與捷徑，保留 app-data；刪除模型、歷史或
生成物須另由明確的程式內／使用者檔案操作完成。這也讓安裝失敗後可安全重試。

## 本機 unsigned smoke build

直接執行下列命令只產生本機測試 artifact，不能作為公開版；Windows 可能警告：

```powershell
./scripts/Build-WindowsInstaller.ps1 -AllowUnsigned
```

除非明確指定 `-AllowDirty`，腳本拒絕未提交來源。它會跑 GPU-free release gate、
只建立 NSIS target，將版本／commit 專屬目錄寫入 `artifacts/windows/`，
包含 installer、standalone executable、授權文件、確定性的依賴清單、
`SHA256SUMS.txt` 與 `build-provenance.json`。

## 公開版建置

從乾淨、已提交的 revision 建免費 unsigned-public 候選版：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = '<安全保存的 updater 私鑰路徑>'
./scripts/Build-ReleaseCandidate.ps1 -AllowUnsignedPublicRelease
```

它不需要付費 Windows 憑證，但必須明確設定與內嵌公鑰相符、妥善保管的 Tauri
updater 私鑰。流程會執行完整 GPU-free gate，封裝雙語 release notes、已知問題、
GPU 矩陣及驗收手冊。Provenance 記錄 `releaseMode: unsigned-public`；文件會
揭露沒有 Windows Authenticode 簽章。`-AllowUnsignedLocalSmoke` 永不具備
可發布資格。Windows 可能警告或依本機政策阻擋；GitHub 與 updater 簽章都不會
提供已驗證的 Windows Publisher。

Artifact 驗證器會用內嵌公鑰與版本密碼學驗證 updater `.sig`，並核對檔案雜湊
與 provenance。完整人工驗收與發布順序見[候選版驗收手冊](release-candidate-runbook.zh-TW.md)。

若日後選擇 CA-signed 模式，省略 `-AllowUnsignedPublicRelease`，把有私鑰的
Code Signing certificate 匯入 Windows 憑證存放區，並提供：

```powershell
$env:BEATFORGE_WINDOWS_CERTIFICATE_THUMBPRINT = '<40 位十六進位 SHA-1 thumbprint>'
$env:BEATFORGE_WINDOWS_TIMESTAMP_URL = '<憑證提供者 timestamp URL>'
./scripts/Build-ReleaseCandidate.ps1
```

提供者需要 RFC 3161 時加上 `-TimestampUsesRfc3161`。腳本會檢查憑證、
私鑰、期限與 Code Signing EKU；Tauri 對程式與 NSIS installer 簽章，再要求
`Get-AuthenticodeSignature` 對兩者回報 `Valid`。

兩種公開模式都拒絕未提交來源、過時 notices、測試失敗及已存在的 artifact 目錄。
Signed 模式另要求有效 Authenticode 與 timestamp。Checksum／provenance
記錄 Git commit、來源狀態、工具版本、lockfile hash、artifact hash、發行模式及
可用時的 signer 身分；不記錄私鑰或憑證密碼。

## 一次性乾淨環境驗收

使用沒有 Beatforge 安裝和 app-data 的新 Windows 帳號、VM 或 Windows Sandbox。
複製**完整** artifact，以一般測試使用者執行：

```powershell
./scripts/Test-WindowsInstallerLifecycle.ps1 `
  -ArtifactDirectory '<artifact directory>' `
  -DisposableEnvironment `
  -AllowUnsigned
```

Signed 版省略 `-AllowUnsigned`。腳本拒絕非空白 profile，檢查安裝、首次啟動、
重新安裝、解除安裝與 app-data sentinel 保留。它會留下資料夾供檢查，之後
請移除一次性帳號／VM。

人工紀錄還須確認：

1. Publisher／SmartScreen 的實際顯示；unsigned-public 不宣稱已驗證發行者，
   也不建議關閉 Windows 安全功能。
2. 開始功能表啟動與可選桌面捷徑。
3. 沒有 WebView2 的電腦能下載 bootstrap；首次中斷後可重試。
4. 首次 runtime／模型下載都進入 app-data，不進安裝目錄。
5. 生成測試譜面；重啟、重裝後歷史與輸出仍在。
6. 解除安裝後程式和捷徑已移除，但 runtime、模型、設定、歷史與輸出保留。
7. 核對 checksum、provenance、updater 簽章；signed 版另核對兩個
   Authenticode 簽章與 timestamp。

階段 9.8 可在其他 gate 與乾淨環境驗收通過後，採明確揭露的 unsigned-public
模式；CA 簽章留作未來改善。驗收的 JSON 與人工紀錄應與候選版一起保存。
