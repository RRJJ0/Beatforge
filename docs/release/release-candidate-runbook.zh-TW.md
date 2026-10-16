# Windows 候選版驗收手冊

[English](release-candidate-runbook.md) · [開發者文件](../development/README.zh-TW.md)

以下必要檢查未全部通過並記錄前，不得公開、標記正式版本或上傳 `latest.json`。
本機 unsigned／dirty smoke artifact 永遠不能當公開候選版。

## 建置與來源紀錄

1. 審閱 release notes、已知問題、第三方聲明、FFmpeg 授權，以及固定模型的
   重新散布權與訓練來源說明。
2. 提交所有預定內容，從精確 revision 的乾淨 checkout 建置。
3. 明確設定與程式內公鑰相符、可長期保管的 updater 私鑰；勿存入 repository 或
   artifact。若採 Windows 已簽章模式，另需可信任的 Code Signing certificate
   與 timestamp URL；明確揭露的 unsigned-public 模式不需要這兩項。
4. 免費的 unsigned-public 模式執行
   `./scripts/Build-ReleaseCandidate.ps1 -AllowUnsignedPublicRelease`；
   具 CA 憑證的 signed 模式省略此參數。`-AllowUnsignedLocalSmoke` 不可發布。
   保留測試輸出、Git commit 與完整 artifact。核對 updater 密碼學簽章、
   `SHA256SUMS.txt`、`build-provenance.json` 及 `releaseMode`；signed 模式
   還要核對兩個 Authenticode 簽章、Publisher 與 timestamp。產生 checksum
   後不得修改任何檔案。
5. unsigned-public Release 必須顯著說明沒有 Windows Authenticode 簽章，
   Windows 可能顯示安全警告；不得宣稱已驗證 Publisher，也不要叫使用者關閉
   防護。先把 installer 與 `.sig` 上傳到 `latest.json` 指定位置，確認可以
   存取及驗證後才發布 `latest.json`。目前程式的 `/releases/latest/` 端點
   需要已公開、非 prerelease 的 GitHub Release；draft 或 prerelease 不會成為 latest。

## 乾淨 Windows 環境驗收紀錄

記錄 Windows build、帳號／VM、GPU／驅動、Git commit、artifact checksum、
日期、測試者、實際結果與證據位置。生命週期腳本只能在一次性帳號／VM 執行，
不可用有真實 Beatforge 資料的帳號。

| 檢查 | 結果／證據 |
| --- | --- |
| 安裝；記錄 signed／unsigned 模式下的 Publisher／SmartScreen 實際顯示 | 待驗 |
| 首次啟動與 runtime／模型設定 | 待驗 |
| 生成至少一筆測試譜面 | 待驗 |
| Validation 結果與警告可理解 | 待驗 |
| 預覽並匯入 osu!；檢查 editor 的音訊與 timing | 待驗 |
| 匯出診斷並檢查沒有歌曲／私密資料 | 待驗 |
| 重新安裝後設定、模型、任務與輸出保留 | 待驗 |
| 解除安裝後使用者資料保留、程式移除 | 待驗 |
| 從舊版經真實 HTTPS 端點更新至新版 updater-signed artifact | 待驗 |
| 更新／下載中斷與離線復原 | 待驗 |
| 降版與未來 schema 被拒絕且不寫入資料 | 待驗 |

更新驗收要使用兩個不同 SemVer build 與實際內嵌公鑰；只改 `latest.json`
不算更新測試。保留舊版 installer、兩版 manifest 與 app-data 備份。

## 回退與修復

候選版失敗時，先停止發布其 manifest，保留 artifact 與證據。只有在版本不低於
本機資料 schema 時才可恢復前一個可用 installer；程式會阻擋不安全的降版。
不得為了啟動舊版而刪除 `%APPDATA%\com.beatforge.desktop`。應先備份資料，
再以修復後的新版本或經審查的資料遷移處理。Runtime／模型失敗可在 **模型**
頁重試，已驗證的 cache 可離線重用。修復 installer 時重跑同一個已驗證版本，
並確認歷史和輸出保留。

正式發布批准需另行記錄；建置腳本無法證明 GUI 行為、osu! Editor 品質或公開
更新端點可用。
