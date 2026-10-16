# 階段 9 GPU 與驅動支援矩陣

[English](stage-9-gpu-support-matrix.md) · [使用者系統需求](../user/requirements.zh-TW.md)

這是 Beatforge 0.1 GPU 支援的發布參考文件；對應的 machine-readable 資料為
`gpu-support-matrix.json`。政策測試會檢查門檻與 Rust preflight 實作是否一致。

## 支援政策

| 分級 | NVIDIA 驅動 | Compute capability | 專用顯存 | 行為 |
| --- | ---: | ---: | ---: | --- |
| 建議 | 560.76 以上 | 8.0 以上 | 8 GiB 以上 | CUDA 生成與原生 BF16；主要發布目標。 |
| 最低 | 560.76 以上 | 7.5 以上 | 2–8 GiB | 允許 CUDA；Ampere 以前 GPU 使用較慢的 FP32 fallback。建議關閉其他 GPU 程式並減少單次難度數。 |
| 部分測試 | 驅動符合，但 probe 無法取得 VRAM 或 compute capability | 不明 | 不明 | 可嘗試 CUDA，但不宣稱此組合受支援；回報時請匯出診斷。 |
| 不支援 | 低於上述任一最低門檻 | — | — | 阻擋 CUDA 套件安裝與任務建立，並顯示復原建議；CPU 模式仍可供慢速相容性測試。 |

不論分級，可用 VRAM 低於 **1536 MiB** 時會暫時阻擋 CUDA 任務；關閉大量
使用 GPU 的程式後可重新診斷。驅動門檻依 Beatforge 固定的 CUDA 12.6 以上
PyTorch wheel 基線制定；新驅動可能選擇更新的 wheel。

BF16 能力依 NVIDIA compute capability 8.0 以上推定，並與 PyTorch runtime
就緒狀態分開顯示。Mapperatorinator V32 在已記錄的 Turing 基線改用 FP32。
硬體分級描述容量；`Ready` 另確認 Python、PyTorch、Mapperatorinator、模型與
CUDA runtime 已安裝並可載入。

## 實機紀錄

| 分級／情境 | Windows／硬體 | 驅動／精度 | 安裝／模型載入 | Peak VRAM | 生成證據 | 穩定性與限制 |
| --- | --- | --- | --- | ---: | --- | --- |
| 建議／主要開發機 | Windows 11 build 26300；RTX 4070、Ada、compute 8.9、12282 MiB | 595.97、610.88／原生 BF16 | 通過／通過 | 1175.2 MiB allocated | 單難度 154.6–172.5 秒；重複 2–4 難度總推論 278.5–998.6 秒 | 多筆正式任務完成並持久化。實際時間依歌曲長度與難度變動；allocated VRAM 不等於驅動保留顯存。 |
| 最低兼低顯存 | Windows（當時未記錄精確 build）；MX450、Turing、compute 7.5、2048 MiB | 573.91／FP32 fallback | 通過／通過 | 1636.4 MiB allocated | 兩次單難度：221.941 與 212.201 秒 | 兩次輸出可重現且通過；未實測多難度，建議關閉其他 GPU 程式並減少單次難度。 |

最低配置同時是低顯存情境，因此兩部實機涵蓋階段 9.7 的最低、建議和低顯存
驗收類別。RTX 4070 時間取自 2026-10-16 的本機 provenance，沒有複製歌曲、
標題、演出者、路徑、seed 或譜面內容。MX450 紀錄保存在
`stage-5-gpu-acceptance.md`。

## 尚未實測的組合

- NVIDIA 3–7 GiB VRAM 雖符合政策，但尚未獨立實機驗收；屬最低容量級，
  不視為建議硬體。
- Blackwell 及後續架構理論上能力相容，但本專案未實測。
- Windows 10、ARM64 Windows、筆電 hybrid-GPU 切換、eGPU、多 GPU 選擇與
  虛擬化 GPU 未實測。
- 不區分 Studio 與 Game Ready 驅動；依版本及實際 CUDA 可用狀態判定。
- AMD、Intel、Apple GPU 不是 CUDA 支援目標；CPU 模式不是發布效能目標。

## 畫面提示與復原

**模型** 頁會顯示 Rust 政策回傳的分級、架構、總／可用 VRAM、BF16、摘要與建議。
硬體 preflight 失敗會停用 CUDA backend 選項，並在 CUDA 依賴安裝及建立任務前
再次檢查，避免可預測的驅動／顯存失敗進入推論。

隱私安全的診斷 ZIP 在 `system.json` 包含相容性結果。硬體回報可提供 diagnostic ID
與矩陣分級，不應附歌曲或譜面。

## 新 GPU 的驗收步驟

1. 在支援的 Windows x64 帳號安裝目前 Beatforge，開啟 **模型** 頁。
2. 記錄 GPU、架構、總／可用 VRAM、驅動、分級與 BF16 狀態。
3. 安裝或修復 runtime／模型，執行環境診斷與 worker health。
4. 生成單難度與至少雙難度各一筆，只記錄耗時、peak allocated VRAM、狀態與穩定錯誤碼。
5. 再執行一次單難度，重新啟動程式並確認 provenance 保留。
6. 匯出隱私安全診斷，僅以實機證據更新中英文矩陣和 JSON；自動 fixture 不得列為實機驗收。

如果宣稱支援的分級發生模型載入失敗、CUDA OOM、精度無聲切換、worker 重複
crash 或 UI 與矩陣不一致，該組合必須阻擋發布。
