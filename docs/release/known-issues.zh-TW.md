# Beatforge 0.1.0 已知問題與限制

[English](known-issues.md) · [首頁](../../README.zh-TW.md)

- 自動驗證通過後，生成譜面仍可能需要人工檢查 timing 與製譜品質。驗證失敗時
  可能仍顯示匯入選項供檢查，這不代表通過品質審核。
- 生成依賴固定版本的 Mapperatorinator V32 與首次的大型下載；離線使用需要
  先前已驗證的本機 cache。
- CUDA 支援依硬體而異。已實測顯示卡與未驗證組合見
  [GPU 支援矩陣](../testing/stage-9-gpu-support-matrix.zh-TW.md)；CPU 模式較慢。
- 缺少 WebView2 的電腦可能需要在安裝時連網下載。
- 未簽章公開版沒有 Windows 驗證的發行者；SmartScreen 可能警告，受管理電腦
  的政策也可能拒絕。放在 GitHub 或具備獨立 updater 簽章都不會消除這項限制。
- 正式 GitHub updater endpoint 與已簽章的 `latest.json` 已公開，但尚未記錄在
  乾淨環境透過該正式端點完成 `0.0.9` → `0.1.0` 更新的證據；發布前的隔離 HTTPS
  updater 流程已有使用者回報通過。

回報問題可使用程式的診斷匯出，預設不包含歌曲內容；分享前仍請檢查 ZIP
是否含有個人資訊，只附上必要的重現細節。
