# 開發者文件

[English](README.md) · [使用者首頁](../../README.zh-TW.md)

原本偏向開發規劃的 README 已完整保存在
[階段與決策詳細紀錄](roadmap.zh-TW.md)，另有較精簡的
[英文階段摘要](roadmap.md)。這些是歷史開發紀錄，不代表每一項都已成為目前功能。

## 程式與架構

- `app/src/`：Vue UI、store、桌面服務與雙語介面。
- `app/src-tauri/src/`：Rust 桌面指令、任務、儲存、安裝、驗證、更新與診斷。
- `app/src-tauri/resources/python/`：Python worker 與固定版本 AI 整合。
- [Mapperatorinator 整合](../architecture/mapperatorinator-integration.md)、
  [worker 協定](../protocol/worker-protocol.md)、
  [輸出驗證](../architecture/output-validation.md)。
- [UI 線框圖](../ui/README.zh-TW.md)。

## 測試與發布

- `scripts/Test-Release.ps1`：不需 GPU 的測試、lint、type-check 與政策檢查。
- `scripts/Build-ReleaseCandidate.ps1`：從乾淨且已提交的 revision 建立已簽章或
  明確標示 unsigned-public 的 Windows 候選版。
- [候選版驗收手冊](../release/release-candidate-runbook.zh-TW.md)、
  [Windows installer](../release/windows-installer.zh-TW.md) 與
  [更新／復原](../release/updates-and-recovery.zh-TW.md)。
- [GPU 實機矩陣](../testing/stage-9-gpu-support-matrix.zh-TW.md)與
  [授權政策](../legal/licensing-and-distribution.zh-TW.md)。

深入的協定、架構與測試紀錄仍保留原本技術語言；根目錄 README 連到的使用者
文件均提供英文與繁體中文版本。
