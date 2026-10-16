# Beatforge

[English](README.md)

Beatforge 是在 Windows 本機使用 Mapperatorinator，從音樂產生
**osu!standard 譜面草稿**的桌面程式。它協助完成環境安裝、生成、驗證、預覽與
osu! 匯入；不取代 osu! 編輯器、不會自動上傳，也不保證譜面可玩或符合排名資格。

## 開始使用

1. 發布版本上線後，從本專案的 [Releases](https://github.com/RRJJ0/OsuBeatforge/releases)
   下載 Windows x64 安裝程式與 `SHA256SUMS.txt`。先確認[系統需求](docs/user/requirements.zh-TW.md)
   與[已知問題](docs/release/known-issues.zh-TW.md)。
2. 安裝並開啟 Beatforge。首次設定會下載固定版本的 AI runtime 與模型，需預留網路、
   磁碟空間及等待時間；模型不包含在安裝程式內。
3. 在 **模型** 頁確認或修復環境；於 **設定** 選擇運算裝置、osu! 路徑、輸出偏好與語言。
4. 在 **生成** 選擇音訊、填寫歌曲名稱和演出者、新增一個或多個難度，然後按
   **生成**。背景圖片、影片與同一首歌的 `.osu` 參考譜面皆為選用。
5. 在 **任務** 查看進度，檢查驗證報告與預覽後匯入 osu!；**歷史紀錄**可找回舊任務。
   分享前請在 osu! 編輯器人工檢查音訊、timing、物件與難度。

操作細節、驗證失敗、匯入、疑難排解與診斷匯出請看[完整使用指南](docs/user/guide.zh-TW.md)。

## 下載與分享前須知

- 目前支援 Windows 10/11 x64，生成模型為 Mapperatorinator V32 的 osu!standard。
  請看[系統需求與 GPU 支援摘要](docs/user/requirements.zh-TW.md)。
- 公開版本可能**沒有 Windows Authenticode 簽章**。這代表安裝程式沒有 Windows
  驗證的發行者，Windows 可能警告或依電腦政策封鎖；放在 GitHub 不會改變這點。
  請核對 release 的來源 revision 與 SHA-256，不要為 Beatforge 關閉 Windows
  安全功能。獨立的 updater 簽章會保護下載更新，但不等同 Windows 發行者簽章。
- 生成與驗證不能代替人工製譜。如果要分享，須揭露 Mapperatorinator／AI 使用情形、
  查核當時的 osu! 規則，並確認音樂和媒體的使用權。詳見[分享與上傳說明](docs/user/sharing.zh-TW.md)。
- Beatforge 不會把歌曲或譜面上傳到 Beatforge 服務；首次設定會連線至上游下載來源。
  分享診斷 ZIP 前請自行檢查。詳見[隱私與授權說明](docs/legal/licensing-and-distribution.zh-TW.md)。

## 更多資訊

- [版本說明](docs/release/release-notes.zh-TW.md) · [已知問題](docs/release/known-issues.zh-TW.md)
- [使用指南](docs/user/guide.zh-TW.md) · [系統需求](docs/user/requirements.zh-TW.md)
- [開發者文件與專案歷史](docs/development/README.zh-TW.md)
- [MIT 授權](LICENSE) · [第三方聲明](THIRD_PARTY_NOTICES.zh-TW.md)

Beatforge 為獨立專案，與 osu! 官方沒有從屬或背書關係。
