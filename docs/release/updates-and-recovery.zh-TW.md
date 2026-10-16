# 更新、降版保護與離線復原

[English](updates-and-recovery.md) · [開發者文件](../development/README.zh-TW.md)

## 信任界線

Beatforge 使用 Tauri updater 並強制檢查簽章所綁定的版本。每個 Windows
release 都有 NSIS installer、updater `.sig` 與靜態 `latest.json`。Manifest
雖透過 HTTPS 取得，但 HTTPS 與 manifest 本身不是足夠的信任依據；下載的
installer 必須通過程式內公鑰的簽章驗證，Tauri 才會執行。Updater 私鑰不能提交。

Updater 簽章與 Windows Authenticode 不同。本機 smoke build 可使用 ignored
development key；公開版（包含明確揭露的 Windows unsigned installer）需明確設定、
安全保存且與內嵌公鑰相符的 updater 私鑰。unsigned-public 不需要 CA 核發的
Windows 憑證，但 Windows 也不會顯示經驗證的 Publisher。

只能在第一次公開 updater release **之前**更換 development key：

```powershell
cd app
pnpm tauri signer generate --ci -w src-tauri/.release/beatforge-updater.key
```

把產生的公鑰內容寫入 `plugins.updater.pubkey`，私鑰放在有備份和存取控制的
地方。公開版上線後遺失或更換私鑰，需要明確的遷移計畫。

## 更新流程與中斷復原

**設定** 頁可檢查 GitHub Release 的靜態 manifest。只接受嚴格較新的
SemVer；相同版本與降版會被拒絕。套用前，Beatforge 會在 app-data 原子寫入
`update-state.json`，記錄來源版本、目標版本與嘗試時間。下次啟動時，若已
成為目標版本則記錄成功；仍為舊版則標記可重試的失敗。

Windows 以目前使用者範圍的 passive 模式安裝更新。失敗或中斷應保留前一版
程式及所有 app-data。`latest.json`、`.sig`、installer checksums 與
provenance 由 `scripts/Build-WindowsInstaller.ps1` 同時產生。

## 降版與資料遷移界線

NSIS installer 禁止無提示降版，updater 只接受較新版。資料庫會在執行任何
migration／manifest sync 前，獨立檢查最高 `schema_migrations` 版本。
如果使用者手動讓舊程式讀取新資料，啟動會中止並顯示兩個 schema 版本，
且不寫入資料。

設定文件及受管理 package manifest 也會拒絕程式無法理解的新版 schema。
Migration 為 additive 且可重複執行；runtime、模型、任務、生成輸出與 osu!
handoff cache 都在應用程式安裝目錄外，更新不會刪除。

## 離線與修復

更新檢查可關閉。網路失敗會回報 `offline`，不應使程式無法啟動。已安裝且驗證
過的 runtime／模型與內建固定 package manifest 可繼續使用；健康的本機元件
不應只因開啟程式或開始生成而重新下載。

套件下載使用 `.part`、由已驗證 byte range 續傳，啟用前檢查大小與 SHA-256。
Runtime／模型安裝使用 staging，直到新版驗證通過才替換舊的健康目錄。重啟時
會清理殘留 staging，並復原中斷的 backup。失敗套件可在 **模型** 頁重試；
已驗證 cache 可離線重用，遺失或損毀時需重新下載。

## 驗收

先執行 GPU-free gate，再建立具 updater 簽章的 Windows unsigned artifact：

```powershell
./scripts/Test-Release.ps1
./scripts/Build-WindowsInstaller.ps1 -AllowUnsigned
```

更新生命週期測試需提供兩個不同版本，經測試版實際設定會讀取的隔離 HTTPS
端點傳送。正式 `/releases/latest/` 不會選到 GitHub draft 或 prerelease。
驗證正常更新、下載中斷、installer／簽章竄改被拒絕、離線啟動、未來 schema
無寫入地被拒絕，以及 runtime／模型受損後的修復。階段 9.8 需重跑公開版
發行路徑，並記錄 Windows Authenticode 是 signed 還是明確的 unsigned。
