# Beatforge 0.1.0 候選版說明

[English](release-notes.md) · [首頁](../../README.zh-TW.md)

狀態：release candidate。公開前仍需通過發布測試、updater 簽章驗證與乾淨
Windows 環境驗收；發布者必須註明實際使用的 Windows 簽章模式。

Windows Authenticode unsigned：如果 GitHub Release 採用免費的未簽章發行方式，
Windows 不會顯示經驗證的發行者，並可能警告或依本機政策阻擋下載或安裝。Tauri
updater 的獨立簽章會保護更新檔，但不會讓 Windows 執行檔取得 Authenticode 簽章。
安裝前請核對來源 revision 與 SHA-256，不要為 Beatforge 關閉 Windows 安全功能。

## 本版內容

- Windows x64、目前使用者範圍的 NSIS 安裝程式與 Tauri 更新 metadata。
- 固定版本的 Mapperatorinator V32 生成、多難度任務、驗證、預覽與 osu! 匯入。
- 可選背景圖片／影片，以及各難度的生成設定。
- 受管理的 runtime／模型下載、離線重用、診斷匯出及英／繁中介面。

## 需求與界線

支援目標為 Windows 10/11 x64。首次設定會下載固定的 runtime 與模型；生成需要
足夠的本機磁碟與記憶體。CUDA 相容性及已實測硬體見
[系統需求](../user/requirements.zh-TW.md)與
[詳細 GPU 矩陣](../testing/stage-9-gpu-support-matrix.zh-TW.md)。不支援 Windows 7、
ARM64 與 32-bit Windows。

生成譜面需要人工檢查；驗證通過或匯入成功不代表符合排名要求。請遵守
[AI 揭露與 osu! 上傳說明](../user/sharing.zh-TW.md)。

測試前請閱讀[已知問題](known-issues.zh-TW.md)與
[候選版驗收手冊](release-candidate-runbook.md)。`build-provenance.json` 記錄精確的
來源 revision、工具版本與雜湊。
