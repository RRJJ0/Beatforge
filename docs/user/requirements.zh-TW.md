# 系統需求與 GPU 支援

[English](requirements.md) · [首頁](../../README.zh-TW.md)

Beatforge 目前以 **Windows 10/11 x64** 為目標平台，使用固定版本的
Mapperatorinator V32 生成 **osu!standard** 譜面。不支援 Windows 7、
Windows ARM64 與 32-bit Windows。已有 Windows 11 實機紀錄；Windows 10
雖列為目標，但尚未完成獨立實機驗收。

首次設定需要網路、足夠的磁碟空間下載受管理的 Python runtime 與模型；缺少
WebView2 時安裝程式可能需要連線補齊。模型權重不包含在 installer，已驗證的本機
runtime／模型則可離線重用。

| CUDA 分級 | NVIDIA 驅動 | GPU 能力／顯存 | 預期情況 |
| --- | --- | --- | --- |
| 建議 | 560.76 以上 | Compute capability 8.0 以上、至少 8 GiB VRAM | 主要 BF16 目標；RTX 4070 已實測。 |
| 最低 | 560.76 以上 | Compute capability 7.5 以上、至少 2 GiB VRAM | 可以執行；較舊 GPU 可能使用較慢的 FP32。MX450 2 GiB 已實測。 |
| 不支援 CUDA | 驅動太舊、capability 低於 7.5 或顯存低於 2 GiB | — | CUDA 安裝與任務會被阻擋；CPU 模式可供較慢的相容性測試。 |

如果**可用顯存少於 1536 MiB**，Beatforge 會暫時阻擋 CUDA 任務；可關閉其他
大量使用 GPU 的程式後重試。符合分級門檻不保證每首歌或多難度任務都不會顯存不足。
NVIDIA 3–7 GiB、Blackwell、hybrid／multi-GPU 與 Windows 10 尚未完成各自的
實機驗收。AMD／Intel GPU 不是 CUDA 目標；CPU 模式並非效能目標。

**模型** 頁會顯示偵測到的 GPU、驅動、顯存、分級、BF16 與環境就緒狀態。
完整[實機證據與限制](../testing/stage-9-gpu-support-matrix.md)保存在開發者文件。
