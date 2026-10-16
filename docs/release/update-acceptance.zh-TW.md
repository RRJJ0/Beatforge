# 隔離 HTTPS updater 驗收

[English](update-acceptance.md) · [候選版驗收手冊](release-candidate-runbook.zh-TW.md)

此測試在一次性 Windows 帳號或 VM 使用 `https://localhost`，不需要公開主機或 GitHub Release。它會以兩個不同的 SemVer 版本，實際測試 HTTPS 下載與 updater 簽章；版控中的正式端點和內嵌公鑰都不變。測試版與正式版共用 Beatforge 的應用程式 ID 與資料位置，**絕對不要裝在日常使用的帳號**。

1. 在開發機上，利用已驗證的 `0.1.0` 候選 artifact 產生測試包。工具會建置指向測試端點的 `0.0.9` 安裝檔。舊版未做 Windows Authenticode 簽章；新版候選安裝檔必須有有效的 Tauri updater 簽章。如果 `48765` 已被使用，可選其他空閒連接埠。

   ```powershell
   ./scripts/New-UpdateAcceptanceBundle.ps1 `
     -NewArtifactDirectory './artifacts/windows/Beatforge-0.1.0-<commit>' `
     -ManifestUrl 'https://localhost:48765/latest.json' `
     -InstallerUrl 'https://localhost:48765/Beatforge_0.1.0_x64-setup.exe'
   ```

2. 將整個 `artifacts/update-acceptance/<timestamp>/` 資料夾複製到有 Node.js 的一次性 Windows 帳號或 VM。在第一個終端執行內附的 `Start-UpdateAcceptanceServer.ps1 -DisposableEnvironment`，並保持執行。若帳號已有 Beatforge 安裝或資料，腳本會拒絕啟動。它會建立有效期兩天的 localhost 憑證，只在此帳號的 **CurrentUser** 憑證存放區暫時信任；透過 IPv4／IPv6 loopback 只提供 installer、`.sig`、`latest.json`。正常按 Ctrl+C 停止後，會移除憑證與暫存私鑰檔。

   ```powershell
   ./Start-UpdateAcceptanceServer.ps1 -BundleDirectory . -DisposableEnvironment -Port 48765
   ```

3. 在同一測試帳號開第二個終端，確認真正的 HTTPS 端點與本機已驗證檔案一致。這一步**還不是**更新測試。

   ```powershell
   ./Test-UpdateAcceptanceEndpoint.ps1 -BundleDirectory .
   ```

4. 安裝 `old/` 內的舊版。到「設定 → 更新」檢查 `0.1.0`，按「安裝並重新啟動」。更新前後用 `(Get-Item "$env:LOCALAPPDATA\Beatforge\Beatforge.exe").VersionInfo.ProductVersion` 檢查安裝檔的 **ProductVersion** 分別為 `0.0.9`、`0.1.0`；目前側欄版本文字是固定值，不能當作驗收證據。確認程式可啟動、測試任務或資料標記仍在。保留 `old/`、`hosting/latest.json`、`original-candidate-latest.json`、測試資料備份及結果。
5. 在第一個終端按 Ctrl+C 停止伺服器，確認顯示的 localhost 憑證指紋已從 CurrentUser 的 Root 與 My 存放區移除。若強制終止導致清理未執行，只在一次性帳號手動移除**該確切指紋**，並清除 TEMP 下產生的 `beatforge-update-acceptance-*.pfx`。在[候選版驗收手冊](release-candidate-runbook.zh-TW.md)記錄測試帳號／VM、來源與候選包 commit、`acceptance.json` 的 SHA-256、時間、畫面／日誌和通過或失敗。

伺服器僅綁定 `127.0.0.1` 與 `::1`，其他電腦無法連線。HTTPS 檔案檢查通過，**不等於** updater 實際更新已通過。不要把測試包上傳到正式 GitHub Release。
