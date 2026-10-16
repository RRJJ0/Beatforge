# 授權與散布政策

[English](licensing-and-distribution.md) · [首頁](../../README.zh-TW.md)

## 發布界線

Beatforge 採 MIT 授權；第三方元件仍適用各自的授權。Release artifact 只能包含
桌面程式、已編譯前端、圖示、`LICENSE`、`THIRD_PARTY_NOTICES.md` 與同一 commit
產生的套件清單。

Python runtime、Mapperatorinator 原始碼、slider 套件、PyTorch、torchaudio、
imageio-ffmpeg／FFmpeg 及 Mapperatorinator 模型權重都屬於**首次設定下載元件**。
除非另有人工審查、記錄來源、授權文字、聲明及必要的 source offer，否則不得放入
Beatforge installer。

模型 repository 雖宣告 MIT，但簡短的 model card 只列出兩個訓練資料集，尚不足以
讓 Beatforge 獨立宣稱有權重新散布模型。因此目前只從上游抓取固定 revision，
不把模型鏡像成 Beatforge release asset。

imageio-ffmpeg wheel 含 FFmpeg 執行檔。FFmpeg 通常採 LGPL 2.1-or-later，
但某些建置功能可能使二進位檔受 GPL 約束。每個 release candidate 都要保存實際
`ffmpeg -version` 設定並人工確認義務；不能只靠套件名稱判斷。

## 可重現的套件清單

更動 lockfile 或依賴套件後，在 repository 根目錄執行：

```powershell
./scripts/Update-ThirdPartyInventory.ps1
./scripts/Test-ThirdPartyLicenses.ps1
```

產生器結合 Cargo metadata、pnpm 正式環境授權報告、附雜湊的 Python
requirements 與已審查的 Python 授權對照表，移除本機路徑與時間戳並排序。
測試會在暫存路徑重新產生 JSON，要求與提交的版本逐位元組一致。

`Cargo.lock`、`pnpm-lock.yaml`、Python requirements、Mapperatorinator
lock manifest 與套件清單一起識別依賴集合。`THIRD_PARTY_NOTICES.md` 說明人工
審查的散布界線，並隨應用程式提供。

## 發布阻擋條件

- 依賴套件沒有授權宣告或必要的 Python 人工對照。
- Lockfile 改變卻未重新產生套件清單。
- Artifact 遺漏專案授權、第三方聲明或套件清單。
- 下載限定元件、模型、歌曲、譜面、背景圖片或影片被放進 installer。
- Mapperatorinator、slider 或模型固定版本與已審查 manifest 不一致。
- 未人工審查實際 imageio-ffmpeg／FFmpeg 建置設定。
- 授權要求提供原始碼、重新連結材料、署名或其他聲明，但 release 未提供。

自動測試能找出漂移與明顯遺漏；發布者仍需親自審核套件清單與實際授權條款。

## 網路與隱私界線

Beatforge 不會把歌曲、譜面、圖片、影片或生成輸出傳到 Beatforge 服務。首次設定
會向套件清單所列上游下載 Python runtime、Python 套件、Mapperatorinator 與模型；
這些主機依自身政策可能收到一般請求資訊，如 IP 位址與 user agent。提交的套件
清單不含本機路徑、使用者名稱與時間戳。診斷匯出採獨立 allowlist，預設不包含
媒體或譜面內容。

## 對外原則

Beatforge 生成的譜面必須揭露 Mapperatorinator／AI 輔助；建議先以 WIP 分享、
取得回饋，並在更改狀態前查核當時 osu! 排名規則。使用者須自行確認所有歌曲、
圖片、影片、hitsound 等輸入素材的使用權。Beatforge 並非 osu! 官方產品，
也未獲官方背書。
