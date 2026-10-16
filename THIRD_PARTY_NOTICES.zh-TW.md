# Beatforge 第三方聲明（繁體中文）

[English — authoritative notices](THIRD_PARTY_NOTICES.md) · [首頁](README.zh-TW.md)

此文件說明 Beatforge 0.1.0 的第三方元件；英文原文與上游授權文件為正式依據。
Beatforge 本身採 [MIT License](LICENSE)。此授權不會把第三方軟體、模型權重、
媒體或使用者內容改為 MIT 授權。本說明僅供資訊參考，不是法律意見。

版本化、可由程式讀取的完整清單為 `third_party/dependency-inventory.json`，
記錄確切套件版本、宣告授權、可用的作者／專案網址，以及元件是隨 Beatforge
出貨，還是在使用者開始設定後下載到私人 runtime。Rust 與前端清單由
`Cargo.lock`、`pnpm-lock.yaml` 重新產生；Python 清單依附雜湊的
requirements 檢查。

## 安裝後下載的元件

下列元件**不包含**在 Beatforge installer：

- **Mapperatorinator**：固定 commit
  `0e2b0e387aab4b35c64b0b11b12d47578dea7587`，Copyright (c) 2024
  OliBomby，MIT。 [原始碼與授權](https://github.com/OliBomby/Mapperatorinator/tree/0e2b0e387aab4b35c64b0b11b12d47578dea7587)。
- **slider**：固定 commit `ea28c484a0a542d6e094e25aa02e1529ff2c8313`，
  LGPL-3.0-or-later。[原始碼與授權](https://github.com/OliBomby/slider/tree/ea28c484a0a542d6e094e25aa02e1529ff2c8313)。
  Beatforge 不修改或靜態連結 slider；pip 把它裝進使用者的 Python 環境，
  使用者可更換為相容版本。
- **Mapperatorinator-v32 模型權重**：固定 revision
  `74f22583400d259bf424819e11027c17933efe54`，
  [repository metadata](https://huggingface.co/OliBomby/Mapperatorinator-v32/tree/74f22583400d259bf424819e11027c17933efe54)
  宣告 MIT。Model card 提到 `project-riz/osu-beatmaps` 與
  `project-riz/osu-beatmap-tags`，但資料不夠讓 Beatforge 獨立批准重新散布；
  因此首次設定後由上游下載，不鏡像或內含於 release artifact。
- **CPython 3.10、PyTorch 2.10.0、torchaudio 2.10.0、imageio-ffmpeg
  0.6.0**：安裝於受管理 runtime。來源及授權見依賴清單。imageio-ffmpeg
  wheel 包含平台 FFmpeg 執行檔；每個 release candidate 均須從
  `ffmpeg -version` 記錄並審查實際 LGPL／GPL 設定。

下載元件仍受各自授權約束。程式將它們存於受管理資料目錄並驗證固定版本或
雜湊；Beatforge 不主張擁有這些元件。

## 隨桌面程式出貨的元件

Tauri／Rust 執行檔與編譯後的前端包含清單中 `rust` 和 `npm` 元件。
宣告的授權含 MIT、Apache-2.0、BSD、ISC、Unicode、zlib 等寬鬆授權。
清單中的套件名稱、版本、授權表示式、作者與上游連結屬於本聲明的一部分；
上游版權聲明與授權檔案仍具權威性。

Release 封裝必須包含本聲明英文原文、Beatforge `LICENSE` 與由同一 release
commit 產生的確切 `dependency-inventory.json`。缺漏／未知授權、禁止散布條款、
lockfile 變更、未審查 FFmpeg build，或模型／runtime 檔案進入 installer，
都會阻擋發布。自動產物有助審查，但不能取代人工判斷。

## AI 生成、osu! 與使用者內容

Beatforge 透過 Mapperatorinator 生成譜面內容。生成成果應清楚揭露 AI 輔助，
審查期間建議以 Work In Progress 分享。現行 osu! 排名規則要求 hit objects、
hitsounds 與 timing 由人類直接建立，對 ranked 譜面的 AI 生成媒體也有限制。
規則可能變動；上傳前使用者須查核當時的 osu! 規定。

歌曲、美術、影片、hitsounds 等素材的使用與上傳權利由使用者負責。Beatforge
為獨立專案，未受 osu!、ppy Pty Ltd、OliBomby 或上述第三方專案從屬、背書或贊助。
