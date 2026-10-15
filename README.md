# osu! AI Beatmap Desktop（暫名）

一款在使用者電腦上執行的 osu! AI 輔助製譜桌面應用程式。使用者可以選擇歌曲或既有 beatmap、設定遊戲模式與目標難度，透過 Mapperatorinator 在本機產生可繼續於 osu! Editor 編輯的 `.osu`／`.osz` 草稿。

> 本專案定位為「AI 輔助製譜工具」，不是完整的 osu! Editor 替代品，也不保證生成內容符合 ranked 標準。使用者發布譜面時應主動揭露 AI 的使用情形。

## 專案狀態

目前處於產品設計與技術規劃階段。第一個開發里程碑是使用 draw.io 完成桌面介面的資訊架構、使用流程與低保真線框圖，再以 mock data 實作 Vue 3 UI。

## 產品目標

- 讓不熟悉 Python、CUDA 或命令列的玩家也能使用 Mapperatorinator。
- 將環境檢查、模型安裝、生成參數、進度與錯誤整理成清楚的桌面體驗。
- 在本機完成音訊分析與模型推論，不將歌曲上傳到外部服務。
- 產生適合在 osu! Editor 中繼續修改的 beatmap 草稿。
- 保留生成模型、參數、seed 與輸出紀錄，使結果可追蹤、可重現。
- 將桌面產品層與 AI 模型層分離，方便未來替換模型或加入其他生成器。

## 設計概念

### 1. Local-first

音訊、譜面和生成結果預設只保存在使用者電腦上。GPU 運算由本機執行，不依賴雲端帳號、伺服器佇列或遠端儲存。

### 2. 對一般玩家友善

介面使用玩家能理解的詞彙，例如「目標星數」、「Pattern 偏好」和「低顯存模式」，不直接暴露 Hydra 指令或原始模型參數。技術參數收納於進階模式。

### 3. 漸進式複雜度

預設畫面只顯示完成一次生成所需的選項；熟悉 Mapperatorinator 的使用者可以展開進階設定，調整 descriptors、CFG scale、seed、timing context、LoRA 等參數。

### 4. 長時間任務可觀察

生成過程可能持續數分鐘，因此使用者必須能看到目前階段、經過時間、進度、GPU 狀態與錯誤原因，也必須能安全取消任務。

### 5. 失敗時提供解決方式

應將 Python traceback、CUDA error 或 FFmpeg error 轉換成可理解的訊息，例如「顯示卡記憶體不足」，並提供低顯存模式、關閉其他 GPU 程式或修復 runtime 等建議。

### 6. 不取代 osu! Editor

第一版不實作完整 beatmap 編輯器或可遊玩的 osu! 模擬器。應專注於生成、檢查、封裝和開啟 osu! Editor，避免重複開發已有的成熟功能。

### 7. 可替換的模型層

Vue UI 不直接依賴 Mapperatorinator 的 CLI 格式。桌面核心與 Python worker 透過固定協定溝通，使未來升級 Mapperatorinator、切換 checkpoint 或加入其他模型時，不必重寫整個前端。

## 預期使用流程

1. 第一次啟動時檢查作業系統、GPU、驅動程式、磁碟空間與 osu! 路徑。
2. 安裝或修復 Python AI runtime、FFmpeg 和模型權重。
3. 選擇音訊、`.osu` 或 `.osz` 作為輸入。
4. 設定遊戲模式、目標難度、風格和輸出選項。
5. 檢查預估顯存、模型與生成設定。
6. 建立任務並查看推論進度。
7. 自動驗證輸出的 `.osu` 並封裝為 `.osz`。
8. 查看結果摘要，下載、開啟輸出資料夾或交由 osu! 匯入。
9. 在 osu! Editor 中繼續人工調整。

## osu! 分享與上傳原則

以下原則最後核對於 2026-10-14；osu! 規則可能更新，實際分享前仍應重新確認 [Beatmap submission](https://osu.ppy.sh/wiki/en/Beatmapping/Beatmap_submission)、[Ranking criteria 的 AI policy](https://osu.ppy.sh/wiki/en/Ranking_criteria#ai-policy)、[Beatmap submission rules](https://osu.ppy.sh/wiki/en/Rules#beatmap-submission-rules) 與 [Beatmapping help](https://osu.ppy.sh/wiki/en/Help_centre/Beatmapping)。

- Beatforge 只負責在本機生成、驗證與交由 osu! 匯入，不會代替使用者將 beatmap 上傳至 osu! 官網。
- 完整或部分使用 Mapperatorinator 生成的 beatmap，建議以 **Work In Progress（WIP）** 分享並尋求測試／意見，不應以 Pending 表示準備進入排名流程。WIP 是負責任的產品建議，不是 osu! 目前針對所有 AI 譜面的明文強制上傳類別。
- 現行 Ranking Criteria 要求 hit objects、hitsounds 與 timing 由人類直接建立，不得使用生成式工具；因此 Mapperatorinator 生成的內容不符合 Ranked AI policy。揭露 AI 或事後人工修改，不會自動使原始生成內容取得 Ranked 資格。
- 必須在 beatmap 網站 description 清楚揭露 Mapperatorinator／AI 實際生成的範圍。`.osu` 的 `Tags` 只用於搜尋，不能取代公開揭露；[Mapperatorinator README](https://github.com/OliBomby/Mapperatorinator#readme) 本身也要求使用者主動揭露 AI 使用情形。
- osu! 上傳會以每個 difficulty 的 `Creator` 判斷擁有者；上傳前必須確認它與上傳者的 osu! username 完全一致，包含大小寫。目前 Beatforge 仍固定輸出 `Creator: Beatforge AI`，在加入可編輯譜師欄位前，使用者必須於 osu! Editor 手動修正所有 difficulty。
- WIP 仍必須遵守內容與權利規則。使用者應確認歌曲、背景、影片、hitsound 與其他資源具有可供 osu! 分享的權利，並依規則提供必要來源；Beatforge 的本機生成與 validation 不代表取得內容授權。
- Beatforge 與 osu! 官方沒有從屬或背書關係；validation 通過只表示輸出符合本專案已實作的結構／安全檢查，不代表 osu! 官方接受、可 Ranked 或內容權利已清除。

建議放在 osu! beatmap description 的英文揭露範本：

```text
This beatmap was generated using Mapperatorinator through Beatforge.

AI was used to generate the timing, hit objects, and hitsounds.
Uploaded as Work in Progress for testing and feedback.
This beatmap is not intended for ranking.

Mapperatorinator:
https://github.com/OliBomby/Mapperatorinator
```

若實際只使用部分生成能力，應按真實情況修改第二行，不得把 AI 生成的 timing、hit objects 或 hitsounds 描述為完全人工製作。

## 主要功能

### 首次啟動與環境檢查

- 偵測 Windows、CPU、RAM、NVIDIA GPU 與可用顯存。
- 檢查驅動程式、PyTorch/CUDA、FFmpeg、AI runtime 與模型狀態。
- 偵測 osu!stable 與 osu!(lazer) 安裝位置。
- 一鍵安裝、驗證或修復 runtime 與模型。
- 顯示下載大小、進度、版本和檔案 checksum。

### Beatmap 生成

- 從音訊建立全新 beatmap。
- 從既有 `.osu`／`.osz` 建立新難度。
- 選擇 standard、taiko、catch 或 mania。
- 設定目標星數、難度名稱、風格、年代與 seed。
- 選擇是否生成 timing、hit objects 與 hitsounds。
- 模式專屬設定，例如 mania key count 與 hold note ratio。
- 基本／進階設定切換與參數預設組合。

### 任務管理

- 任務佇列與一次一個 GPU 推論任務。
- 顯示等待、準備音訊、載入模型、生成、後處理及封裝狀態。
- 取消、重試、複製設定與重新生成。
- 顯示經過時間、進度、GPU 使用狀態與可理解的錯誤。
- 保留必要 log 供診斷，不讓一般 log 汙染前後端通訊。

### 結果與驗證

- 顯示 AR、OD、CS、HP、BPM、物件數量與估算 star rating。
- 驗證 `.osu` 格式、時間排序、座標、timing points 與 slider 資料。
- 封裝 `.osz`。
- 開啟輸出資料夾、交由 osu! 匯入或啟動 osu! Editor。
- 保存模型版本、參數、seed、生成時間與警告。

### 歷史、模型與設定

- 以 SQLite 保存生成任務和結果索引。
- 搜尋、篩選、刪除或以相同設定重新生成。
- 下載、更新、驗證、切換或移除模型。
- 管理輸出、模型、runtime 與暫存資料夾。
- 設定主題、語言、通知、預設參數與 osu! 路徑。
- 匯出診斷報告與 log。

## MVP 範圍

第一個可用版本包含：

- Windows 10/11。
- NVIDIA GPU。
- 本機 Mapperatorinator 推論。
- 音訊輸入與既有 beatmap 輸入。
- 基本 standard 生成流程；其他模式保留介面與擴充點，再逐步驗證。
- Runtime 與模型安裝／檢查。
- 單一 GPU 任務佇列、進度、取消與錯誤顯示。
- `.osu` 驗證與 `.osz` 輸出。
- 生成歷史和 osu! 開啟／匯入流程。

第一版不包含：

- 完整 beatmap editor。
- 完整可遊玩的 osu! 預覽器。
- 雲端帳號與同步。
- 社群分享或 osu! 官網自動上傳。
- 多 GPU 排程。
- LoRA 訓練介面。
- 插件市場。

## 系統架構

```text
┌─────────────────────────────────────┐
│ Vue 3 / TypeScript UI               │
│                                     │
│ Generate · Jobs · History           │
│ Models · Settings · Diagnostics     │
└──────────────────┬──────────────────┘
                   │ Tauri commands / events
┌──────────────────▼──────────────────┐
│ Tauri 2 / Rust Desktop Core         │
│                                     │
│ File dialogs · Process lifecycle    │
│ Runtime/model manager · osu! paths  │
│ SQLite · Updates · Notifications    │
└──────────────────┬──────────────────┘
                   │ JSON Lines over stdin/stdout
┌──────────────────▼──────────────────┐
│ Python AI Worker                    │
│                                     │
│ Mapperatorinator · PyTorch · CUDA   │
│ FFmpeg · Validation · .osz export   │
└─────────────────────────────────────┘
```

### 責任邊界

#### Vue 前端

- 畫面、表單、路由與互動狀態。
- 任務進度、結果、歷史和錯誤呈現。
- 將使用者操作轉成型別化 request。
- 不直接啟動 Python、不拼接 shell command，也不直接管理模型檔案。

#### Tauri／Rust 核心

- 原生檔案與資料夾選擇器。
- Python worker 的啟動、監控、取消和重啟。
- 驗證所有路徑與生成 request。
- Runtime、模型下載、版本與 checksum 管理。
- SQLite、系統通知、應用更新和 osu! 路徑偵測。
- 將 worker 事件轉送為 Tauri events。

#### Python worker

- 載入 Mapperatorinator 與模型。
- 音訊預處理和 GPU 推論。
- Beatmap 後處理、驗證與封裝。
- 將進度與結果以 JSON Lines 寫入 stdout。
- 將診斷 log 寫入 stderr 或 log file。

## 使用框架與工具

| 分層 | 技術 | 用途 |
|---|---|---|
| 桌面殼 | Tauri 2 | 原生視窗、檔案系統、程序管理、安裝與更新 |
| 前端 | Vue 3 | 桌面 UI 與元件組合 |
| 前端語言 | TypeScript | 型別安全的畫面、事件與 IPC 資料模型 |
| 建置工具 | Vite | 前端開發伺服器與 production build |
| UI 元件 | Naive UI | 表單、選單、Dialog、Progress、Table 與暗色主題 |
| 狀態管理 | Pinia | Runtime、任務、模型、設定與歷史狀態 |
| 路由 | Vue Router | Generate、Jobs、History、Models、Settings 頁面 |
| 桌面核心 | Rust | 系統整合、sidecar lifecycle、安全檢查與 IPC |
| AI worker | Python | Mapperatorinator、PyTorch、音訊處理與輸出生成 |
| AI 模型 | Mapperatorinator | 由音訊或既有譜面生成／修改 beatmap |
| 本機資料 | SQLite | 任務、參數、結果索引、模型與設定資料 |
| 通訊 | JSON Lines | Rust 與長時間運行 Python worker 的雙向訊息 |
| UI 規劃 | draw.io | User flow、wireframe、頁面狀態與元件規格 |
| 測試 | Vitest、Vue Test Utils、Rust tests、pytest | 各分層單元與整合測試 |

實際套件版本會在建立專案時鎖定，並以 lockfile 和相容性測試管理，不直接追蹤所有依賴的最新版。

## Worker 通訊概念

桌面核心與 Python worker 使用一行一個 JSON 物件的協定。stdout 僅用於協定事件，診斷資訊寫入 stderr。

建立任務：

```json
{"type":"start_job","job_id":"job_123","audio_path":"D:\\Music\\song.mp3","output_path":"D:\\osu-ai\\job_123","gamemode":0,"difficulty":5.5,"year":2024,"descriptors":["skillset/jumps","style/clean"],"seed":12345}
```

回報進度：

```json
{"type":"progress","job_id":"job_123","stage":"generating","percent":43,"message":"Generating hit objects"}
```

完成任務：

```json
{"type":"completed","job_id":"job_123","output_path":"D:\\osu-ai\\job_123\\result.osz","warnings":[]}
```

所有 request 與 event 都應包含 `protocol_version`。正式實作時需定義 schema、錯誤碼、取消語意、版本相容規則和最大訊息長度。

## UI 資訊架構

建議使用固定左側導覽：

```text
Generate
Jobs
History
Models
Settings
```

Diagnostics 收納於 Settings 中，避免第一版導覽過度膨脹。

### draw.io 設計檔

預計建立：

```text
docs/ui/osu-ai-desktop-wireframes.drawio
docs/ui/exports/
```

`.drawio` 檔應包含下列頁面：

1. Information Architecture
2. First-run Setup Flow
3. Generate — Empty State
4. Generate — Audio Selected
5. Generate — Advanced Settings
6. Generation Confirmation
7. Active Job
8. Job Failed
9. Generation Result
10. Job History
11. Model Manager
12. Settings and Diagnostics
13. Dialogs and Notifications
14. Component and State Reference

每個主要畫面至少設計以下狀態：

- Empty
- Loading
- Ready
- Disabled
- Validation error
- Runtime/model unavailable
- GPU out of memory
- Task cancelled
- Task completed with warnings

draw.io 原始檔必須納入版本控制；必要畫面另匯出 SVG 或 PNG，供 issue、README 和開發比對使用。

## 開發步驟

### 階段 0：產品定義與 draw.io UI 設計

目標：在撰寫 UI 程式碼前確定資訊架構、核心流程與所有關鍵狀態。

工作項目：

- 整理 MVP、非目標與專有名詞。
- 建立主要 persona 與核心 user journey。
- 使用 draw.io 畫出資訊架構和頁面流程。
- 完成首次啟動、建立任務、生成中、完成及失敗流程。
- 完成所有主要頁面的低保真 wireframe。
- 定義 UI tokens：色彩、字級、間距、圓角和狀態顏色。
- 定義 Naive UI 元件對照表。
- 匯出關鍵畫面供審查。

交付物：

- `docs/ui/osu-ai-desktop-wireframes.drawio`
- 關鍵畫面的 SVG／PNG exports
- 頁面清單、狀態清單與操作註解

驗收條件：

- 使用者能從首次啟動走到成功開啟生成結果。
- runtime 缺失、模型缺失、GPU 不支援、取消與失敗都有明確畫面。
- 每個操作都有下一步、返回或復原方式。

### 階段 1：Vue UI Prototype

目標：先以假資料完成可操作的完整前端，不連接 Rust 或 Python。

工作項目：

- 建立 Vue 3、TypeScript、Vite、Pinia、Vue Router 與 Naive UI。
- 實作 App Shell、側邊導覽和暗色／亮色主題。
- 建立 Generate、Jobs、History、Models 和 Settings 頁面。
- 以 mock service 模擬環境檢查、模型下載與生成進度。
- 完成表單驗證、空狀態、錯誤狀態和通知。
- 建立共用 TypeScript domain types。
- 對照 draw.io 進行 UI review。

交付物：

- 可在瀏覽器執行的 UI prototype
- mock jobs、models、diagnostics 資料
- 基礎 component tests

驗收條件：

- 不啟動後端也能完整演示主要流程。
- 重新整理後可恢復必要的 mock 狀態。
- 所有主要表單皆有輸入限制與錯誤提示。

### 階段 2：Tauri 桌面殼與本機能力

目標：將 UI 轉為可安裝的 Windows 桌面應用程式。

工作項目：

- 初始化 Tauri 2 與 Rust commands。
- 加入原生檔案／資料夾選擇器。
- 建立應用程式資料、cache、log 與 temp 路徑策略。
- 實作系統資訊和 GPU 基礎偵測。
- 實作 osu!stable／osu!(lazer) 路徑設定。
- 建立 SQLite migration 與 repository layer。
- 建立權限、CSP 與 allowlist。

交付物：

- 可安裝並啟動的 Windows build
- Tauri command types
- SQLite schema 與 migration

驗收條件：

- 可安全選取音訊和輸出資料夾。
- 可重新啟動並保存設定。
- 前端不能任意執行 shell command 或讀取未授權路徑。

### 階段 3：Worker 協定與假 Worker

目標：在接入大型模型前驗證 subprocess、IPC、取消和復原機制。

工作項目：

- 定義 versioned JSON Lines schema。
- 建立輕量 mock worker，模擬進度、成功、失敗與卡住。
- 實作 Rust process manager。
- 實作 stdout parser、stderr log、timeout 與 event forwarding。
- 實作取消、強制終止和意外退出處理。
- 加入 app 重啟後的 interrupted job recovery。

交付物：

- Worker protocol 文件
- Mock worker
- Rust/Vue IPC integration tests

驗收條件：

- UI 可正確顯示所有模擬階段。
- 取消後不殘留 worker process。
- 不合法或未知版本訊息不會造成應用程式崩潰。

### 階段 4：Runtime 與模型管理

目標：讓沒有 Python 開發環境的使用者也能完成安裝。

目前進度：

- [x] 4.1：建立 package manifest schema、SQLite 安裝狀態與合法狀態轉換。
- [x] 4.2：下載、續傳、checksum 與磁碟空間檢查。
- [x] 4.3：獨立 Python runtime 管理。
- [x] 4.4：FFmpeg、PyTorch 與 CUDA 相容性處理。
- [x] 4.5：模型安裝、更新、移除與修復。
- [x] 4.6：空機及異常情境驗收。
  - [x] 暫存 clean profile 與異常情境自動化驗收。
  - [x] 實體空白 Windows 完整下載與重啟驗收。

工作項目：

- 建立獨立、固定版本的 Python runtime。
- 安裝並驗證 PyTorch、CUDA 相容套件與 FFmpeg。
- 建立模型 manifest、版本、URL、大小與 SHA-256。
- 實作下載進度、中斷續傳、驗證、更新與移除。
- 實作磁碟空間檢查和安裝修復。
- 建立 GPU／runtime 診斷報告。

交付物：

- Runtime manager
- Model manager
- 診斷與修復流程

驗收條件：

- 全新 Windows 環境不需使用者自行安裝 Python。
- 中斷下載後可安全恢復。
- checksum 錯誤或空間不足時不會留下可被誤用的安裝狀態。

#### 4.4 Runtime 相容矩陣

桌面程式使用私有 Python 環境安裝固定版本的 PyTorch 2.10.0、torchaudio 2.10.0 與 imageio-ffmpeg 0.6.0，不修改系統 Python 或全域 PATH。PyTorch wheel 內含 CUDA runtime，因此使用者不需另外安裝 CUDA Toolkit；桌面程式只依 NVIDIA 驅動選擇經測試的 wheel channel。

| NVIDIA Windows 驅動 | 建議套件 | PyTorch index |
|---|---|---|
| 580.00 以上 | CUDA 13.0 | `cu130` |
| 570.65–579.xx | CUDA 12.8 | `cu128` |
| 560.76–570.64 | CUDA 12.6 | `cu126` |
| 更舊、無法辨識或沒有 NVIDIA GPU | CPU fallback | `cpu` |

安裝前會檢查私有 Python 與 pip；安裝後會實際 import PyTorch／torchaudio、執行 FFmpeg 版本檢查，並確認 CUDA wheel 能看見 GPU。FFmpeg 執行檔由 `imageio-ffmpeg` 的 Windows wheel 提供，後續正式 worker 應以診斷回傳的絕對路徑設定 `IMAGEIO_FFMPEG_EXE`，避免依賴使用者的系統 PATH。

#### 4.5 模型套件

MVP 鎖定官方 `OliBomby/Mapperatorinator-v32` repository 的 commit `74f22583400d259bf424819e11027c17933efe54`，首個套件只安裝 osu!standard checkpoint。模型 manifest 分別記錄 `model.safetensors`、`config.json`、`generation_config.json` 與 `tokenizer.json` 的固定 URL、大小和 SHA-256，總下載量約 830 MB。

下載支援逐檔續傳與整體進度；所有檔案通過大小及 checksum 驗證後，才會從 staging directory 原子切換成可用版本。更新保留舊版本直到新版本啟用成功，修復會重新下載缺少或損毀的檔案，移除操作只允許刪除應用程式管理的 model/cache 目錄。

#### 4.6 驗收

自動化驗收使用隔離的暫存 application profile 與小型 fixture，不修改真實 AppData，也不下載正式權重。測試涵蓋首次安裝生命週期、下載／安裝中斷復原、checksum 損毀、空間不足、更新失敗回滾、manifest 更新及越界刪除防護。完整矩陣與實體空機步驟見 `docs/testing/stage-4-runtime-model-acceptance.md`。

### 階段 5：Mapperatorinator 整合

目標：以正式 Python worker 完成第一個端到端 beatmap 生成。

目前進度：

- [x] 5.1：鎖定 Mapperatorinator 上游版本與整合邊界。
- [x] 5.2：安裝 Mapperatorinator 原始碼與其固定 Python 依賴。
- [x] 5.3：建立正式 Python worker bootstrap、health check 與 JSON Lines 通訊。
- [x] 5.4：建立生成 request adapter、參數驗證與本機路徑解析。
- [x] 5.5：接入真實推論流程、模型載入與階段進度回報。
- [x] 5.6：錯誤分類、取消、GPU／subprocess／暫存資源清理。
- [x] 5.7：生成 provenance、可重現性紀錄與端到端 GPU 驗收。

子項目範圍：

#### 5.1 上游版本與整合邊界

- 固定 Mapperatorinator commit，不直接追蹤 `main`。
- 記錄 source、模型、設定檔、授權及本專案 adapter 的版本關係。
- 決定只透過 adapter 使用 inference API，不讓 Vue 或 Rust 依賴 Hydra CLI 細節。
- 明確排除訓練、Web UI、MaiMod 與非 MVP gamemode。

鎖定結果與 adapter 責任邊界記錄於 `docs/architecture/mapperatorinator-integration.md`；可由程式驗證的來源、模型 revision、archive checksum 與授權資訊記錄於 `app/src-tauri/resources/manifests/mapperatorinator-v32.lock.json`。

#### 5.2 正式 Python 環境

- 將固定版本原始碼安裝至 application-managed runtime。
- 鎖定 requirements 與 transitive dependencies，避免啟動時臨時更新。
- 處理 Git-based dependency，使空白 Windows 不需安裝系統 Git。
- 驗證必要 module 可 import，且 Mapperatorinator source／dependency 版本一致。

實作結果：AI environment 的安裝按鈕會先完成階段 4 的 PyTorch／FFmpeg，再下載具 SHA-256 驗證的 Mapperatorinator source archive，並將 source 安裝至 application data 下的版本化 runtime。Python 依賴使用 Windows x64／Python 3.10 專用 lock；41 個直接與遞迴套件全部固定版本及 artifact hash，pip 以 `--require-hashes --no-deps --no-build-isolation` 安裝，PyTorch／torchaudio 則繼續由 GPU backend 選擇器獨立管理。Hydra 所需的 `antlr4-python3-runtime==4.9.3` 只有 source distribution，因此不能全域限制為 wheel；它仍受版本與 archive hash 約束，且建置不會另開可下載未鎖套件的 isolation environment。

上游 `slider @ git+https://...` 已改為固定 commit 的 HTTPS source archive，因此目標 Windows 不需要 Git。安裝器會保留 Mapperatorinator 與 slider 的授權 identity；slider 為 `LGPL-3.0-or-later`，不是 Mapperatorinator 本體的 MIT。為維持 inference-only 邊界，安裝時會套用內容嚴格比對的 re-export patch，避免載入未使用的 training utilities；若 pinned source 不符合預期便中止，不會修改未知版本。

機器可讀的來源／slider／dependency identity 位於 `app/src-tauri/resources/manifests/mapperatorinator-v32.lock.json`；輸入與完整 hash lock 分別位於 `app/src-tauri/resources/python/mapperatorinator-windows-py310.requirements.in` 和 `.requirements.txt`。診斷頁會分開顯示 Mapperatorinator source 與 inference lock，並執行固定版本檢查及 `pip check`；任一項缺少或不一致時環境不會標示 Ready。

#### 5.3 正式 worker 基礎

- 建立 Python worker entry point 與 startup health check。
- 沿用階段 3 的 versioned JSON Lines schema、stderr log 與最大訊息限制。
- 回報 Python、PyTorch、CUDA、FFmpeg、Mapperatorinator 與模型版本。
- 保留 mock worker，供沒有 GPU 的自動化測試使用。

正式 bootstrap 以 embedded resource 隨應用程式發布，啟動前會原子部署到 application-managed `runtime/worker/9/beatforge_worker.py`，並由私有 Python 的 isolated mode 執行。worker stdin／stdout 只接受 protocol v1 的單行 JSON，單則訊息上限 64 KiB；upstream import、traceback 與其他診斷一律重新導向 stderr，Rust 最多保存 256 KiB 至 `logs/production-worker.log`，避免污染 JSON Lines channel。

Models 畫面的 `Worker health` 會啟動一次最長 90 秒的獨立 health process，確認 Python executable、PyTorch／CUDA、受管理 FFmpeg、Mapperatorinator source marker 與五個固定 inference callables，以及 V32 模型的四個必要檔案。回報只檢查模型 identity 與檔案存在性，不會在 5.3 載入約 830 MB 權重。health report 包含 worker／protocol、Python、PyTorch、CUDA、GPU、FFmpeg、Mapperatorinator commit 與 model version；任何缺少項目會回傳結構化 errors 並顯示於 UI。

階段 3 的 Rust mock worker 與故障情境測試仍保留；一般成功生成任務已於 5.5 切換至 production worker，進階選單中的 failure／OOM／hang／crash／malformed 情境仍走 mock worker，供回歸測試使用。

#### 5.4 Request adapter

- 將 domain request 轉換成 Mapperatorinator inference configuration。
- 驗證音訊、beatmap、輸出路徑、gamemode、difficulty、seed 與 descriptors。
- 解析受管理的模型絕對路徑，不接受前端傳入任意 checkpoint path。
- 第一版只開放已驗證參數，其餘設定保留於 adapter 內部。

實作結果：Vue domain request 只傳受控 model package id，不接受 checkpoint 或輸出路徑；目前開放 osu!standard、1.0–8.0 目標難度、0–65535 seed、正／負 descriptors、選用 `.osu` reference、AR、CS 與 hitsound 開關。音訊只接受上游已支援的 MP3、WAV、OGG、M4A、FLAC，descriptor 數量、長度、重複、正負衝突與控制字元皆在 Rust 邊界驗證。

Rust adapter 會從 package database、application settings 與 managed runtime 解析 Mapperatorinator source、V32 model、輸入和 job output 的 canonical absolute paths；source、model、output 必須分別位於 application-managed root，模型四個必要檔案也必須存在。送入 Python 的 command 只含白名單 inference configuration，不包含 mock scenario 或前端 model 字串；Python worker 在載入推論前會再次驗證相同的檔案類型、範圍與固定 V32／gamemode 0 邊界。

#### 5.5 真實推論與進度

- 載入 V32 osu!standard 模型並執行第一個真實 inference。
- 將音訊準備、模型載入、timing、生成及後處理映射成穩定的 worker stages。
- 將上游不穩定的 log 與進度格式隔離於 adapter。
- 將 Mapperatorinator 產出的 `.osu`／`.osz` 路徑交回 Rust；內容驗證留待階段 6。

實作結果：`success` 任務會由 Rust 使用 application-managed Python 啟動正式 worker，依序呼叫固定版上游的 `compile_args`、`setup_inference_environment`、`load_model_with_server`、`get_config` 與 `generate`。推論強制使用已驗證的本機 V32 model、`auto_select_gamemode_model=false`、`use_server=false` 與 offline mode，不允許 Hugging Face 在任務期間解析或下載其他權重；V32 未啟用額外 diffusion checkpoint。

worker 會回報 `adapter_ready`、`loading_runtime`、`configuring`、`loading_model`、`preparing_audio`、`generating` 與 `validating_output` 等穩定階段。模型載入和長時間生成期間每 15 秒送出 JSONL heartbeat，Rust 對 production worker 使用一小時無回報 timeout；完成後只接受 managed job directory 內的 `.osu`／`.osz`。音訊解碼固定使用 `imageio-ffmpeg` 診斷出的絕對路徑；由於該 wheel 不附 `ffprobe`，adapter 會略過 pydub 非必要的 probe metadata，仍由 FFmpeg 自動選擇 decoder 與 PCM 輸出。固定版自訂 model config 原本會重新查詢 `openai/whisper-small`；adapter 改用 manifest 已驗證的 V32 `config.json` 內嵌 `backbone_config`，backbone identity 或結構不符便中止，不提供網路 fallback。V32 的實驗性 CUDA graph fast decoder 在 PyTorch 2.10 timing pass 可能把全零機率分布傳給 `torch.multinomial` 並觸發 device-side assert，因此 MVP 固定關閉 `fast_decoder_loop`／`super_timing_fast_loop`，改走上游標準 generation path。Settings 的 `Repair unsafe generated gameplay data` 預設開啟；開啟時，adapter 會透過上游 `update_event_times` 規則解析共用時間與 slider component 插值、丟棄解析後時間倒退的 `TIMED_EVENTS` group，並替非正數 SV 或無效 slider length 使用既有速度基準重新計算。相同時間合法，任何其餘有序物件都不會被移動或排序；處理數量與開關狀態會寫入 warning 及 provenance。關閉時完全不安裝這組內容修正，只保留 Mapperatorinator 原始輸出交由 validation 檢查，失敗產物仍可經明確確認後手動匯入。第一版設定 `export_osz=true`，將音訊與生成譜面封裝成可匯入 osu! 的 `.osz`；reference `.osu` 使用 timing context。完整輸出內容與語意驗證仍屬階段 6。

#### 5.6 失敗與資源生命週期

- 分類 CUDA OOM、driver／wheel 不相容、模型缺失、音訊解碼及上游例外。
- UI 顯示可理解摘要，完整 traceback 僅寫入本機 log。
- 取消時先要求 worker graceful stop，逾時後由 Rust 強制終止 process tree。
- 確認取消、失敗與 app 關閉後不殘留 GPU process 或 job 暫存檔。

實作結果：production worker 會將 CUDA OOM、CUDA driver／runtime 不相容、CUDA 執行錯誤、模型不可用、音訊解碼失敗、輸入檔遺失、Hydra configuration 失敗與未分類上游例外轉成穩定錯誤碼及可理解摘要；完整原始例外與 traceback 只寫入 `logs/job-<id>.log`。UI 仍以既有 job record 顯示錯誤碼、是否可重試及友善摘要，不接收 traceback。

Rust 在任務存活期間保留 worker stdin。取消與 app 關閉會先送出 versioned `cancel_job` JSONL 命令，Python cancellation watcher 以 SIGINT 讓主執行緒解除推論並回傳 `JOB_CANCELLED`；三秒內未離開時，Windows 使用 `taskkill /T /F` 終止 PID 及其 process tree，之後一律 wait/reap。timeout、protocol failure 與 monitor failure 也沿用相同收尾流程，確保 Python／CUDA process 不殘留。

每個失敗、取消、啟動失敗與 app 關閉任務都會刪除 `jobs/<id>` 暫存目錄；retry 開始前與應用程式下次啟動時也會清理由中斷任務留下的目錄。清理只接受 managed jobs root 的直接子目錄，canonical path 不符時拒絕刪除。已完成任務的 `.osu`／`.osz` 不會被生命週期清理移除。worker stderr 改以原始位元組持續 drain 至 log，避免 Windows 系統編碼的 `tqdm` 進度造成 UTF-8 line reader 關閉 pipe 與 Python exit code 120。

#### 5.7 可重現性與端到端驗收

- 保存 source commit、model version、PyTorch backend、完整有效參數、seed 與輸入摘要。
- 使用相同輸入與設定重跑，確認 provenance 完整且結果可追蹤。
- 在支援的 NVIDIA GPU 上從 Vue 建立任務並取得第一個可由後續 validator 處理的輸出。
- 記錄 VRAM、driver、耗時、已知限制與失敗案例。

實作結果：production worker 的成功事件現在必須包含 provenance schema v1；內容保存固定 Mapperatorinator commit、model package/version、worker/protocol、Python/PyTorch/CUDA/GPU/driver、所有 adapter 實際生效的推論參數（包含 seed）、輸入與輸出檔案的大小及 SHA-256、UTC 起訖時間、耗時、CUDA peak allocated memory 與已知限制。`reproduction_key` 對 source/model、完整有效參數和輸入摘要做 canonical JSON SHA-256；相同輸入與設定的重跑會得到相同 key，而每次輸出 hash、時間與 runtime observation 仍可獨立比較。

Rust 會先把 provenance 存入 jobs table 既有的 `result_json`，成功後才發布 completed 狀態；正式 worker 若遺漏紀錄或 SQLite 保存失敗，任務會以穩定錯誤終止，不會留下不可追溯的成功結果。Jobs 畫面會顯示 run key、GPU、effective precision、耗時與 peak VRAM，重新啟動後仍由 SQLite 載入。完整自動化命令、同設定重跑規則、實體 Windows/NVIDIA 流程、開發機硬體與已知失敗記錄見 `docs/testing/stage-5-gpu-acceptance.md`。

工作項目：

- 鎖定並記錄可重現的 Mapperatorinator commit／版本。
- 將 Mapperatorinator CLI 或 inference API 包裝為 worker adapter。
- 建立 request 轉換與參數驗證。
- 回報載入模型、音訊處理、推論和後處理進度。
- 實作 CUDA out-of-memory、音訊錯誤與模型錯誤分類。
- 驗證取消時 GPU 記憶體與暫存檔能正確回收。

交付物：

- Mapperatorinator worker adapter
- 第一個由 UI 產生的 `.osu`／`.osz`
- GPU 相容性測試紀錄

驗收條件：

- 使用者可以從選擇歌曲一路完成輸出。
- 相同模型、參數、輸入與 seed 可追蹤並盡可能重現。
- 失敗時 UI 顯示摘要，完整 traceback 保存在 log。

### 階段 6：輸出驗證與 osu! 整合

目標：確保生成檔案可用，並順利交給 osu! Editor。

目前進度：

- [x] 6.1：固定輸出驗證契約、報告 schema、安全限制與測試樣本。
- [x] 6.2：實作 `.osu` parser 與結構驗證。
- [x] 6.3：實作 timing、hit objects、座標與 slider 語意驗證。
- [x] 6.4：實作 `.osz` 安全解包、完整性驗證與重新封裝。
- [x] 6.5：串接 validation pipeline、工作狀態與 SQLite report。
- [x] 6.6：建立非破壞式修正流程與 repair manifest。
- [x] 6.7：計算 difficulty statistics 並呈現結果摘要。
- [x] 6.8：完成 osu! 開啟／匯入流程與 Windows／osu!lazer 實體驗收。

#### 6.1 輸出驗證契約

Mapperatorinator 的成功事件只代表推論完成；階段 6 將輸出視為不受信任的 candidate，由 Rust 保存原始 artifact、驗證、選擇性建立修正副本並持久化 versioned report。接入 6.5 後，report 包含任何 `error` 的任務不得進入 `completed`，只有 `valid`、`valid_with_warnings` 或安全修正後的 `repaired` 可以交付。

每個 job 固定使用 `raw`、`working`、`validated` 三個 artifact 區域；`raw` 不可覆寫，`working` 可清理，所有修正只能輸出至 `validated`。validation report schema v1 已固定 status、severity、stable error codes、issue context、statistics、repair audit 與 UTC timestamps；Rust 與 Vue 使用對應型別。預設 archive 限制為 256 entries、1 GiB 解壓總量、512 MiB 單檔、64 份 `.osu` 與 16 MiB 單一 `.osu`，並明確拒絕 traversal、absolute path、case collision、link entry 與超限輸出。

自動修正採 deny-by-default；第一版只允許 BOM、行尾、尾端換行、安全 metadata whitespace、明確 scalar spelling 與安全檔名正規化，不自動更改 timing、BPM、hit-object 時間／座標／型別或 slider geometry。完整契約見 `docs/architecture/output-validation.md`，測試矩陣見 `docs/testing/stage-6-validation-matrix.md`；人工 `.osu` fixtures 與可審查的 hostile archive recipes 位於 `app/src-tauri/tests/fixtures/validation`，不含使用者歌曲或真實模型輸出。

#### 6.2 `.osu` parser 與結構驗證

Rust validator 現在提供受 16 MiB 上限保護的檔案與 bytes parser；檔案入口會先檢查 metadata 與 regular-file identity，再配置記憶體讀取。parser 接受 UTF-8、選用 BOM、LF／CRLF、空白與 `//` comments，固定支援 Mapperatorinator 輸出的 `osu file format v14` 與 osu!standard `Mode: 0`。無效 encoding、超限檔案或無法辨識的 header 是停止解析的 fatal issue；其餘問題會盡量一次收集並附上 section 與一基準原始行號。

parser 將 `General`、`Metadata` 與 `Difficulty` 轉成 typed section，驗證必要欄位、唯一 key、有限數值及必要 section；`TimingPoints` 與 `HitObjects` 在 6.2 只驗證最低 CSV 結構並保留 raw value、tokens 與行號，實際 BPM、時間、物件型別、座標和 slider 規則留給 6.3。合法未知欄位與未知 section 會完整保留，重複的已知 section／欄位則視為不明確輸入而拒絕。解析結果可同時包含 partial typed beatmap 與 ordered issues，malformed input 不會 panic，且相同輸入會產生確定性相同的問題順序。

#### 6.3 Timing 與 HitObject 語意驗證

semantic validator 會將 6.2 的 raw timing／object tokens 轉成可供後續 statistics 使用的 typed representation。TimingPoints 會驗證有限 time／beat length、時間排序、meter、sample set、volume、uninherited flag 與正負 beat-length 關係，計算 uninherited BPM 或 inherited slider-velocity multiplier，並要求至少存在一個 uninherited timing point。Difficulty 的 HP、CS、OD、AR 必須介於 0–10，slider multiplier 與 tick rate 必須大於零。

osu!standard hit objects 只接受 circle、slider、spinner 及合法 combo flags；validator 會驗證整數時間與排序、hit-sound flags、起始座標 `0..=512 × 0..=384`、spinner 結束時間，以及 slider 的 B/C/L/P curve、`x:y` control points、repeat 與有限正 pixel length。物件 origin 越界是 error；slider control point 位於可見 playfield 外保留輸出但回報 warning。呼叫端提供音訊長度時，超出已知結束時間的物件會回報 `OBJECT_AFTER_AUDIO_END` warning。6.3 仍不解包 `.osz`、不變更 job 狀態，也不自動修改 playable content。

#### 6.4 `.osz` 安全解包與完整性

Rust archive boundary 使用固定 `zip 8.6.0` 且只啟用 deflate 支援。preflight 會先驗證 `.osz`／ZIP 格式、entry count、宣告大小、壓縮方式、加密與 link-like entry，再拒絕 absolute／UNC／drive／device path、`..` traversal、Windows reserved／ADS 名稱、大小寫或 file-as-parent collision。解包目的地必須是不存在且 parent 已存在的直接子目錄；每個 entry 以 `create_new` 寫入，實際串流 bytes 會再次套用單檔與總量上限並驗證 CRC／宣告大小。任何 extraction error 都會移除由本次操作建立的 working directory。

解包完成後，每份 `.osu` 會通過 6.2 parser 與 6.3 semantic validator，並以 archive-relative、大小寫不敏感索引確認 `AudioFilename`、background/video/sample events 與 custom hit-sample 檔案存在；不安全 reference 或缺少資源會阻止完成。重新封裝只遍歷明確 source root、拒絕 symlink 與輸出覆寫，依穩定順序寫入同目錄 `.partial`，完成後串流重讀、原子啟用，再解至隔離 `.verify` 目錄重新執行完整 parser／semantic／resource validation。驗證失敗時候選輸出及 staging artifacts 一律移除。

#### 6.5 Validation pipeline 與持久化

正式 worker 的 `completed` 事件現在只表示推論結束。Rust 會先將任務切換為 `validating`，確認候選檔仍位於受管理的 job directory 且為 regular `.osu`／`.osz`，再將原檔移入不可覆寫的 `raw`。單一 `.osu` 會依序經過 6.2 與 6.3；`.osz` 則解至一次性的 `working/extracted`，執行 6.4 的 archive、beatmap 與資源 reference 驗證，完成後移除 working data。

沒有 error 的候選檔會以 partial file、flush 與同目錄原子 rename 建立 `validated` 交付副本；含 warning 的報告可成為 `valid_with_warnings`，任何 error 都會進入 `validation_failed`，保留 `raw` 但不建立或發布 output path。`validation-report.json` 以相同方式原子寫入，SQLite schema v3 同步保存完整 report JSON，job API 與 Vue 型別可在重新啟動後還原狀態、issue 數量及 validated artifact identity。若程式在 validation 中被關閉，啟動復原會標示為 `interrupted` 並保留已建立的驗證 artifacts，不再把推論成功直接當作產品成功。

#### 6.6 非破壞式自動修正

repair engine 採 deny-by-default，只會移除 UTF-8 BOM、將 CRLF／CR 正規化成 LF、補上缺少的 final newline、整理 Metadata 已知欄位的外圍空白、正規化可明確解析且數值等價的 General／Editor／Metadata／Difficulty scalar spelling，以及在隔離解包時將安全的 archive path separator 轉為穩定名稱。TimingPoints、HitObjects、BPM、座標、物件型別、slider geometry、difficulty 數值本身及缺少的 metadata 內容都不會被猜測或修改；空白 Creator 仍只回報 warning，不能由程式代填。

任何實際修正都只發生在 `working`，完成後重新執行完整 parser、semantic、archive 與 referenced-asset validation。通過時產生新的 `validated` artifact、`repaired` report status 與 `repair-manifest.json`；manifest 逐項記錄 code、section／field、非敏感 before／after 描述、原因及 UTC 完成時間。若重驗仍失敗，incomplete validated output 與 working data 會移除，manifest 標記 `failed`，job 保持不可完成；`raw` bytes 在成功與失敗路徑都不會改動。

#### 6.7 Difficulty statistics 與結果摘要

驗證成功或安全修正成功後，Rust 會從同一份 typed parser／semantic model 計算 statistics，不重新信任 worker 提供的摘要。單一 difficulty 會回報 circle、slider、spinner 數量、uninherited timing points 的 BPM 最小／最大值，以及從第一個 hit object 到最後一個已知結束時間的 playable duration；slider 尾端會使用當時的 redline beat length、inherited SV、SliderMultiplier、pixel length 與 repeats 推導，spinner 使用明確 end time。

`.osz` 會逐一統計所有 difficulty，再彙總 beatmap count、物件總數與整個 set 的 BPM 範圍，duration 顯示最長 difficulty。統計只會寫入可完成的 validation report，無效輸出維持 `statistics: null`。Jobs UI 會顯示 difficulty 數、objects、BPM 與時間摘要；目前沒有引入與 osu! 官方等價的 difficulty calculator，因此 `starRating` 與 `maxCombo` 明確保留為 `null`，不以近似值誤導使用者。

#### 6.8 osu! 開啟與匯入

Jobs UI 對 completed 且具有 validated artifact 的任務提供 **Show validated output** 與 **Import into osu!**。每次操作都會由 Rust 重新讀取 SQLite，要求 job 為 completed、report 可交付、report job identity 一致，並比對 stored output、`validated/<filename>` relative path 與 canonical filesystem path；symlink、`raw`／`working`、外部路徑、非 `.osu`／`.osz` 或已遺失檔案都會拒絕，不信任前端傳入實體路徑。`validation_failed` 且具有 invalid report 的任務另提供帶警告確認的 **Import failed output anyway**；此命令只允許 report 指定、直接位於該 job `raw/` 下的 regular `.osu`／`.osz`，不會把它標記成 validated 或 completed。

Windows 匯入會優先使用 Settings 明確選擇且檔名為 `osu!.exe` 的執行檔，其次使用偵測到的 stable／lazer executable。手動匯入（包含明確確認的 failed-output import）在找不到 executable 時可交給 Windows `.osu`／`.osz` file association；自動匯入只接受已驗證 executable，不使用 association fallback，並且只在完整 validation 成功後啟動。stable／lazer importer 可能消耗傳入的檔案，因此所有匯入都會先在 application-managed `cache/osu-import` 建立唯一 handoff 副本，只將副本交給 osu!；正式 `validated` 或 `raw` artifact 保持不變，未被 importer 消耗且超過 24 小時的副本會在後續啟動時清理。Windows／osu!lazer 實機驗收已於 2026-10-13 通過；記錄位於 `docs/testing/stage-6-physical-acceptance.md`。

工作項目：

- 實作 `.osu` parser／validator。
- 驗證 metadata、timing、hit objects、座標與 slider。
- 封裝和檢查 `.osz`。
- 計算可取得的 difficulty statistics。
- 開啟 `.osz`、輸出資料夾和 osu!。
- 保留原始輸出並以非破壞方式執行修正。

交付物：

- 驗證報告
- `.osz` exporter
- osu! 開啟／匯入流程

驗收條件：

- 成功結果可由 osu! 開啟。
- 驗證失敗的輸出不會被標示為成功。
- 自動修正均可追蹤，且不覆蓋使用者原始檔。

### 階段 7：歷史、設定與產品完善

目標：完成可日常使用的任務管理體驗。

規劃子項目：

- [x] 7.1：建立正式歷史資料檢視、搜尋、篩選與排序。
- [x] 7.2：完成歷史任務刪除、重新生成與檔案一致性處理。
- [x] 7.3：建立設定預設值、版本化 migration 與匯入／匯出。
- [x] 7.4：顯示磁碟用量並提供安全的 cache／暫存資料清理。
- [x] 7.5：加入桌面通知與背景任務完成／失敗提示。
- [x] 7.6：加入輕量音訊、時間軸與物件密度預覽。
- [x] 7.7：完成繁體中文／英文介面與 accessibility 改善。
- [x] 7.8：執行產品整合、migration 與日常流程驗收。

#### 7.1 歷史檢視、搜尋與篩選

- History 畫面在桌面環境直接使用 SQLite 載入的正式 job records，不再顯示獨立的假歷史資料；重新啟動或手動 refresh 都會還原目前資料庫內容。
- 支援 title、artist、job ID、model 與 audio path 關鍵字搜尋，可與狀態、模型及 today／7 days／30 days 日期條件組合。
- 支援 newest、oldest 與 title 排序，並明確呈現完成、驗證失敗、推論失敗、取消、中斷及執行中狀態。
- 結果採每頁 25 筆分頁，篩選變更會回到第一頁；完成且具有 validated artifact 的紀錄可直接開啟檔案位置或匯入 osu!。
- 畫面統計改由目前正式紀錄計算 stored、completed、average target 與 stopped／invalid 數量，並提供 loading、資料庫錯誤及空結果狀態。

驗收：TypeScript type-check、production build 與 History filter/sort 單元測試通過；重新啟動後仍可查到既有任務，組合篩選與空結果狀態正確，畫面內容與 SQLite 一致。

#### 7.2 歷史動作與檔案一致性

- History 與 Jobs 的刪除操作會先列出影響範圍並要求確認；後端拒絕刪除 queued／running／validating job，只允許清理 job ID 對應的直接 managed directory 與精確命名的 job log，成功後才移除 SQLite record。外部音訊與 reference 永遠不在刪除範圍內。
- History 的 regenerate 會從原 job 的完整 `request_json` 複製輸入、descriptor、reference、難度與 seed，建立新的 job identity 並排入 worker；原 job、artifact、validation report 與 provenance 保持不變。
- Regenerate 會在建立新紀錄前重新驗證音訊與 reference path；遺失或類型不符時回傳包含 saved input context 的可理解錯誤。新 job 若無法送入 queue，僅允許回滾尚未啟動的 queued insert。
- 已完成但輸出遭移動或刪除的紀錄仍留在 History；開啟／匯入會沿用 artifact identity gate 回報無法解析，不會誤開其他路徑。刪除遇到遺失的 job directory／log 可安全重試，越界 ID、非 regular log 與 active state 一律拒絕。
- Rust lifecycle 測試涵蓋 source preservation、新 identity、saved reference 複製、active delete rejection、queued rollback、missing record，以及 job directory／log 的精確清理與越界拒絕。

驗收：操作只影響指定 job；無法解析或位於受管理目錄外的路徑不得刪除，舊任務與 provenance 保持可追蹤。Rust lifecycle、前端單元測試、type-check 與 production build 通過。

#### 7.3 設定預設值、匯入／匯出與 migration

- Settings database value 與匯出檔目前使用 schema v2 文件，包含 `schemaVersion` 與完整 `settings` payload；既有無版本／v1 JSON 會在首次讀取時遷移並重新保存，舊 `fp16` 值同步正規化為 V32 使用的 `bf16`，v1 則補入背景通知預設值。
- 保存與匯入會驗證 device、precision、worker count、必要路徑字串及未知欄位；較新 schema、損壞 JSON、非 UTF-8、超過 256 KiB 或非 `.json` regular file 會在修改設定前拒絕。
- Settings UI 提供 JSON 匯出與兩階段匯入。Preview 逐欄顯示 current、result、`apply`／`ignored`／`reset`／`unchanged` 與原因，確認後才保存。
- output、temporary 與 osu! path 視為機器限定欄位；匯入路徑不存在或不適用時保留目前值並標記 ignored，缺少欄位則使用目前 schema／機器預設值。
- Backup & reset 可恢復單一欄位或經確認後恢復全部預設值；匯出內容不包含歌曲、模型、job、artifact 或其他資料庫內容。
- Rust 測試涵蓋 legacy migration、fp16 migration、版本化輸出、缺少欄位 reset、不存在路徑 ignored，以及較新 schema／未知欄位拒絕。

驗收：設定可跨重啟保存；合法設定可往返匯出／preview／套用，損壞或較新版本檔案會被安全拒絕。Rust settings 測試、前端單元測試、type-check、lint 與 production build 通過。

#### 7.4 磁碟用量與安全清理

- Settings 的 Storage management 會統計 runtime、models、cache、logs 與所有 job 的 raw／working／validated 分類，顯示 bytes、檔案數及最後更新時間；另外列出目前真正符合清理條件的容量。
- cache、七天以上的 regular `.log` 與非執行中 job 的 working 目錄為預設候選；runtime 與 models 只能由既有 Models 流程管理，raw 與 validated 視為使用者成果且預設不選。
- Review cleanup 會重新掃描並按分類顯示 targets、檔案／目錄數與預計釋放容量。raw 或 validated 被明確選取時，仍必須在確認視窗再次勾選成果刪除聲明。
- Rust 清理端只接受固定 category enum，再由 managed-root allowlist 建立 target；每次刪除前重新 canonicalize 並確認不等於 root、不越界、不含 symlink／特殊檔案。queued、running 與 validating job 的所有 artifact 目錄都會排除。
- 清理逐一記錄成功與失敗項目，完成後重新掃描並回傳實際移除的 bytes、檔案／目錄數和最新用量；一個 target 失敗不會擴大刪除範圍。
- Rust 測試涵蓋 preview／cleanup 數量一致、成果預設保留、protected confirmation、active job 排除、七天 log cutoff、symlink 拒絕與完整分類統計。

驗收：預覽數量與實際清理結果一致；清理不影響已安裝模型、執行中任務或使用者未明確選擇的成果。Rust storage 測試、前端單元測試、type-check、lint 與 production build 通過。

#### 7.5 桌面通知與背景狀態

- 使用官方 Tauri notification plugin 監聽既有 `job-event`，針對 completed、validation_failed 與 failed transition 建立通知；cancelled／interrupted 不會打擾使用者。
- 前景時只顯示可關閉的應用內 job notice，不再重複發送 Windows toast；背景或視窗失焦時，只有在 Settings 的 Background job notifications 開啟且系統權限 granted 時才發送桌面通知。
- Settings schema 升級至 v2 並保存通知偏好；介面會顯示目前 Windows permission，權限不可用或被拒絕時維持應用內 notice，不改變 job 結果。
- 通知摘要只使用 artist、title、difficulty 與結果類型，不帶 audio/output path、stage、error message 或 traceback。每個 job 的同一 terminal transition 只提示一次，retry 回到 active state 後才重新允許下一次 terminal notice。
- 點擊應用內 notice 或仍在 Windows toast 畫面上的通知會導向 `/jobs?job=<id>`，對應 job 會自動捲入視野並標示；初始 SQLite refresh 不經 event handler，因此重新啟動不重播舊通知。
- 單元測試涵蓋通知內容隱私、terminal deduplication、retry re-arm，以及 cancelled／interrupted 排除。

驗收：各終止狀態至多通知一次；關閉通知後不再顯示系統通知，重新啟動不重播舊通知。Rust settings migration、前端通知單元測試、type-check、lint 與 production build 通過；Windows toast 需以安裝版進行最後實機驗收。

#### 7.6 輕量結果預覽

- Jobs 與 History 的完成項目提供唯讀 Result preview；後端先沿用 artifact identity gate 確認 job 為 completed 且 validated artifact 仍存在，再解析 `.osu` 或 `.osz`，不接受 raw／validation-failed 輸出。
- difficulty 名稱、物件數、首末物件時間與密度只取自 validated beatmap typed parser；`.osz` 最多 64 個 difficulty、單張 16 MiB，密度最多 240 bins，前端 waveform 最多 320 點。
- 支援多 difficulty 切換、原始本機音訊播放、目前播放位置、點擊時間軸 seek，以及波形與物件密度疊圖；時間軸從 0 開始，因此開頭靜音與第一個物件的位置可直接比較。
- 音訊只在請求該 job 預覽時動態加入 asset protocol 的單檔 allowlist；遺失、非 regular file、不支援格式或大於 128 MiB 時會停用音訊或只略過 waveform，密度預覽與既有匯入流程不受影響。
- 預覽不保存、修正或覆寫 beatmap，也不提供 hit object 編輯；需要調整內容時仍使用 osu! Editor。

驗收：Rust preview tests、前端 waveform tests、type-check、lint 與 production build 通過；播放、seek、difficulty 切換、密度對齊，以及遺失／不支援音訊時仍可匯入，需在 Windows 實際 job 上完成最後驗收。

#### 7.7 雙語與 Accessibility

- 新增 reactive `en`／`zh-TW` 字串目錄，儲存語言設定後，主導覽、頁面標題、生成、任務、歷史、結果預覽、模型及設定主要操作會一起切換；未儲存的選擇不會影響其他頁面。技術名稱、檔案路徑、worker/backend 回傳內容維持原文。
- Settings 文件升級至 schema v3 並保存 `locale`；舊版或 legacy 設定自動補入 `en`，匯入／匯出、單欄 reset 與全域 reset 都包含介面語言。瀏覽器 fallback 另以 localStorage 保存，啟動時同步 `<html lang>`。
- App shell 新增 skip link 與具 landmark 的 main target；icon-only action、switch、預覽 slider 補上可讀 label。時間軸支援滑鼠、方向鍵每次 5 秒、Home 與 End，並提供 `aria-valuemin/max/now`。
- 全域 `:focus-visible` 提供高對比外框；原生／Naive modal 沿用 focus trap 與 Escape close，表單錯誤保留文字 feedback，不只依賴顏色。狀態標籤同時顯示文字。
- 支援 `prefers-reduced-motion`，窄視窗／高縮放縮減 sidebar 與內容邊距；長路徑及結果文字沿用截斷或換行規則。

Accessibility checklist：Tab 可由 skip link 進入主內容；可鍵盤完成選檔、表單、生成、結果預覽、匯入與設定儲存；dialog 開啟後 focus 留在 dialog，Escape 可關閉；所有 icon-only 主要動作有 accessible name；200% 縮放下主要操作不被裁切；中英文切換後 title、label 與提示同步更新。

驗收：settings migration、i18n／既有前端單元測試、type-check、lint、production build 與 Tauri no-bundle build 通過；Windows Narrator、鍵盤 focus trap、200% 縮放及兩種語言長字串需實機完成最後驗收。

#### 7.8 產品整合與日常流程驗收

- History、Settings、storage、notifications 與 preview 已接回既有生成／驗證／匯入流程；完成狀態現在另外回報 `outputAvailable`，實體成果被移走或清理後仍保留歷史，但 UI 會明確標示檔案不存在，且不再顯示預覽、開啟與匯入動作。
- 自動化涵蓋舊 jobs schema、共用 migration marker 修復、legacy／v1／v2 settings 升至 v3、重啟中斷復原、遺失成果、磁碟 reserve、storage preview／cleanup，以及 runtime/model/download/validation/osu! handoff 安全案例。
- Windows release build 已使用既有 application profile 執行無命令列 smoke test：成功還原 19 筆歷史、設定、繁中介面與 runtime 狀態，並開啟真實成果的音訊波形、時間軸與 905-object density preview；7 筆仍存在的 validated deliverable 與遺失成果能正確區分。
- 前端 13 項與 Rust 111 項測試、type-check、lint、Clippy warnings-as-errors、production build 及 Tauri no-bundle release build 通過。完整矩陣、實機記錄、已知限制與後續階段待辦見 `docs/testing/stage-7-product-acceptance.md`。

驗收：一般使用者可從生成、背景等待、通知、歷史查找、預覽到 osu! 匯入完成完整流程；歷史、設定與實體檔案保持一致。

工作項目：

- 實作歷史搜尋、篩選、刪除與重新生成。
- 實作設定匯入／匯出與預設值。
- 加入通知、磁碟用量與 cache 清理。
- 加入輕量音訊、時間軸或物件密度預覽。
- 改善鍵盤操作、focus、對比和 accessibility。
- 完成繁體中文與英文介面。

交付物：

- 完整 MVP UI
- 設定與歷史 migration 測試
- Accessibility checklist

驗收條件：

- 一般使用者無需查看命令列即可完成主要工作。
- 歷史紀錄與實體檔案狀態一致。
- 鍵盤可以操作所有主要流程。

### 階段 8：譜面集與媒體內容

目標：讓單次專案能產生完整的 beatmap set，並支援帶有視覺內容的 `.osz`。

規劃子項目：

- [x] 8.1：同一首歌曲一次建立多個難度，輸出為同一個 beatmap set。
- [x] 8.2：允許選擇並封裝背景圖片。
- [x] 8.3：允許選擇並封裝背景影片或其他 osu! 支援的影片內容。
- [x] 8.4：生成的 `.osz` 使用 `[Artist] - Song name.osz` 格式命名。

#### 8.1 多難度 Beatmap Set

- 讓使用者為同一首歌設定多個目標難度，並建立各自的 difficulty name、生成參數與 seed。
- 共用音訊、歌曲 metadata、背景與其他 set-level 資源，避免在 `.osz` 中重複封裝。
- 每個 difficulty 必須分別執行 validation、statistics 與 provenance 紀錄；單一難度失敗時不得默默污染其他難度。
- 匯出時建立一個包含所有有效 difficulty 的 `.osz`，並在 Jobs／History 顯示整個 set 與各難度狀態。

#### 8.2 背景圖片

- 允許使用者選擇本機圖片作為 beatmap background，並在生成前預覽。
- 驗證檔案格式、大小、尺寸與安全檔名，再複製至受管理的 job 工作目錄。
- 將背景引用寫入每個 difficulty 的 `[Events]`，封裝後確認引用資源存在且可由 osu!／lazer 載入。

#### 8.3 背景影片

- 允許使用者選擇本機影片，設定開始時間並封裝至 `.osz`。
- 驗證格式、檔案大小、時間偏移與資源引用；第一版不自動轉碼或修改原始影片。

實作結果：支援 AVI、FLV 與 MP4，最大 512 MB，開始時間限制為 0 至 3,600,000 ms。檔案會先檢查副檔名與容器 signature，再以固定安全檔名複製到受管理的 job 目錄；封裝時以未壓縮 ZIP entry 保存原始影片，並替每個 difficulty 寫入相同的 `Video` event。驗證流程會確認所有引用都存在。
- 顯示影片對輸出大小及匯入時間的影響，並允許在匯出前停用影片。

#### 8.4 `.osz` 輸出命名

- 預設以 beatmap metadata 產生 `[Artist] - Song name.osz`，取代目前偏向 job identity 的檔名。
- 清理 Windows 不允許的檔名字元、保留名稱、尾端空白與句點，同時避免產生空白或過長檔名。
- 同名輸出不得靜默覆寫既有檔案，應加入穩定的序號或其他可辨識後綴。
- job 內部仍使用不可混淆的 identity 管理 `raw`、`validated` 與 provenance；易讀檔名只用於交付與匯入 artifact。

實作結果：worker 原始產物仍以內部名稱保存於 `raw`；只有通過驗證的 `.osz` 會以 `Artist - Song name.osz` 發布至該 job 的 `validated` 目錄。命名會替換 Windows 禁止字元、處理裝置保留名稱、移除尾端空白／句點，並以 UTF-16 長度限制同時保留 artist 與 title。匯入 handoff 保留易讀名稱，若快取中已有同名檔案則依序使用 ` (2)`、` (3)`，絕不覆寫既有檔案。

初步驗收條件：

- 同一首歌可匯出至少兩個不同難度，osu!／lazer 將它們辨識為同一個 beatmap set。
- 所有 difficulty 共用正確的音訊與背景資源，且個別 validation／statistics 可追蹤。
- 加入圖片或影片後，封裝、重新驗證、重啟還原與匯入流程仍保持非破壞性。
- 輸出的 `.osz` 使用可辨識的 artist／song name 命名，且特殊字元與同名歌曲不會造成匯出失敗或覆寫。

### 階段 9：測試、封裝與發布

目標：在譜面集與媒體內容的基礎結構穩定後，建立可重現、可更新和可支援的 Windows 發布流程。

Repository 公開策略：Stage 9.3 與 9.4 完成前維持 GitHub Private。隱私安全的診斷／資料邊界、完整 Git 歷史掃描、Beatforge 專案 LICENSE、第三方 notices，以及 AI 生成與 osu! 上傳限制說明全部通過審查後，才轉為 Public Preview。公開原始碼不代表 installer 已可正式發布；簽章 installer、更新與 release candidate 仍依 9.5–9.8 的驗收進行。`app/package.json` 的 `private: true` 繼續保留，只用來防止前端套件被誤發布至 npm，與 GitHub repository visibility 無關。

規劃子項目：

- [x] 9.1：建立發布層級的自動化測試基線與固定測試資料。
- [x] 9.2：完成端到端流程、異常復原與長時間穩定性測試。
- [x] 9.3：建立隱私安全的診斷匯出與問題回報流程。
- [x] 9.4：完成第三方依賴、模型與資料授權盤點。
- [x] 9.5：建立可簽章的 Windows installer 與乾淨環境安裝流程。
- [x] 9.6：建立自動更新、降級、離線啟動與環境修復流程。
- [ ] 9.7：建立 NVIDIA GPU、顯存與驅動支援矩陣。
- [ ] 9.8：建立 release pipeline，並完成 release candidate 實機驗收。

#### 9.1 發布測試基線

- 整理前端、Rust desktop core、Python worker 與 protocol 的單元及整合測試入口，提供一個可重現的完整驗證指令。
- 建立不含受版權保護歌曲的最小 audio、`.osu`、圖片、影片、job database 與 validation fixtures；測試只使用隔離的 managed roots，不讀寫使用者正式資料。
- 固定測試 runtime、模型 manifest、資料庫 schema 與設定版本，涵蓋全新安裝及支援中的舊版本 migration。
- 對 installer、更新與 GPU 實機無法在一般 CI 執行的案例建立明確的 manual／hardware 標記，不以假通過取代實機驗收。

驗收：單一命令可重跑不需 GPU 的 release test suite；連續執行結果一致，測試結束後不殘留 process、lock 或越界檔案。

實作結果：新增 `scripts/Test-Release.ps1` 作為 GPU-free release gate，依序執行 fixture policy、前端 type-check／唯讀 lint／unit tests／production build、Rust format／Clippy warnings-as-errors／tests，以及 Python worker tests。每次執行將 `TEMP`／`TMP` 導向 `app/.release-test-runs` 下的唯一目錄，結束時還原環境並只清除經路徑與名稱驗證的本次目錄。`tests/fixtures/release/manifest.json` 會完整列舉 committed fixtures，拒絕越界、遺漏、超過 64 KiB、非 allowlist 副檔名及疑似個人絕對路徑；audio、圖片、影片、archive 與 SQLite fixtures 維持由測試在隔離目錄動態建立。自動化範圍、執行方式與明確延後的 manual／hardware gates 記錄於 `docs/testing/stage-9-release-test-baseline.md`。

#### 9.2 端到端與長時間穩定性

- 自動驗證從首次啟動、runtime／model 準備、建立多難度 job、validation、preview 到 osu! handoff 的主要流程。
- 涵蓋取消、worker crash、timeout、CUDA OOM、validation failure、應用程式重啟、成果遺失與磁碟空間不足等復原情境。
- 執行多首歌曲連續生成與長時間 soak test，記錄 worker process、GPU／系統記憶體、暫存空間、執行時間與失敗原因。
- 確認重試或重新啟動不會重複匯入、覆寫成果、污染其他 job，或讓 queue 與 SQLite 狀態分歧。

驗收：主要 E2E 流程可重複通過；長時間測試沒有持續性記憶體／磁碟成長，所有故障都有可理解且可復原的終止狀態。

實作結果：新增 crate 內的 Stage 9 acceptance test，使用隔離的 synthetic audio／`.osz` 與 in-memory SQLite，實際串起多難度 request、job state、validation、statistics、preview、validated artifact identity 及不覆寫的 osu! handoff；重啟測試會將 queued／running／validating 狀態一致轉為 interrupted，retry 後清除舊 output、report 與 provenance。mock worker process integration test 現在涵蓋成功、可恢復 failure、CUDA OOM、crash、malformed JSONL、hang 終止與連續 job 隔離；`scripts/Test-Stage9Stability.ps1` 可用 1–1000 次迭代執行 bounded soak，並確認測試輸出回收及沒有殘留 worker process。完整自動化矩陣、執行方式、production worker／GPU 長跑步驟與 release blocker 定義見 `docs/testing/stage-9-e2e-stability.md`；GPU 型號與驅動相容性仍由 9.7 實機驗收，不以 mock 測試代替。

#### 9.3 診斷匯出與問題回報

- 提供使用者可預覽後匯出的診斷包，包含應用程式版本、OS、GPU／驅動、runtime／model 狀態、設定 schema、job stage、validation 摘要與已清理的近期 logs。
- 預設排除歌曲、`.osu`／`.osz` 內容、圖片、影片、完整個人路徑、使用者名稱及其他不必要的個人資訊；路徑與自由文字錯誤內容必須經過 redaction。
- 診斷匯出只讀取 allowlist 內的資料，使用版本化 manifest，並在建立檔案前顯示內容與隱私提醒。
- 建立 bug／hardware compatibility issue template，要求提供診斷 ID、重現步驟及預期／實際結果，不要求公開歌曲內容。
- Public Preview 前掃描目前工作樹與完整 Git 歷史，確認沒有曾提交的 token、credential、歌曲／譜面、真實 job database、個人路徑、使用者名稱或其他無法靠刪除最新檔案移除的敏感內容。

驗收：診斷包足以判斷常見安裝、GPU、worker 與 validation 問題，且自動化測試確認其中不含歌曲檔案、beatmap 內容或未清理的個人路徑。

實作結果：Settings 新增診斷匯出卡片與兩階段確認流程；後端以 deny-by-default allowlist 建立 schema v1 ZIP，使用者必須先檢查檔案、大小、排除項目與隱私提醒，勾選確認後才能選擇儲存位置。診斷內容包含 app／OS／CPU／GPU／VRAM／driver、固定 Mapperatorinator commit、runtime／model package 狀態、非路徑設定、匿名 job stage 與 validation code/count，以及近期 log 的大小／行數／invalid UTF-8／已知 stable error code 計數；不包含 raw log 文字、traceback、SQLite、歌曲、`.osu`／`.osz`、圖片、影片、生成成果、完整路徑、username、title、artist、descriptor、seed 或自由文字錯誤。ZIP 內 `manifest.json` 記錄 diagnostic ID、schema、UTC、檔案大小與 SHA-256，輸出拒絕覆寫且失敗會移除 partial。GitHub bug／hardware issue templates 只要求 diagnostic ID、重現步驟與必要硬體資料，並強制確認未附加私人內容。資料邊界與測試策略見 `docs/architecture/diagnostic-export.md`。`scripts/Test-RepositoryPrivacy.ps1` 會掃描所有 reachable Git revisions 的 credential／個人路徑 pattern 與禁止的 media、database、model artifact，並已納入 release gate；2026-10-15 的 44-revision baseline 無敏感內容，僅有 allowlisted app icons、synthetic validation fixtures 與 `C:\Users\You` 泛用 UI placeholder，詳見 `docs/testing/stage-9-privacy-scan.md`。

#### 9.4 授權與第三方 Notices

- 為 Beatforge 選定並加入根目錄 LICENSE，同步 Rust package metadata；未加入 LICENSE 前即使 repository 可見，也不得宣稱為可自由修改與散布的開源專案。
- 盤點 Rust crates、npm packages、Python packages、FFmpeg／媒體相關能力，以及 installer／updater 使用的第三方元件與版本。
- 分別確認 Mapperatorinator 程式碼、模型權重、設定檔、可能涉及的訓練資料聲明與重新散布限制；不能隨 installer 發布的內容改用首次啟動下載。
- 建立可隨版本重現的 dependency inventory、授權文字、copyright notice、source offer／連結與隱私說明。
- 在公開文件中明確說明 AI／Mapperatorinator 使用揭露、建議僅以 WIP 分享、現行 osu! Ranked AI policy、輸入媒體權利責任，以及 Beatforge 與 osu! 官方沒有從屬或背書關係。
- 將禁止散布、授權不明或 notice 缺失視為 release blocker，不以自動掃描結果取代人工確認。

驗收：Beatforge 本身的授權與第三方授權邊界清楚；installer 與 release artifact 內附完整 Third-party notices；所有實際散布的二進位、模型與資源都有可追蹤的授權依據。完成 9.3 隱私驗收後，repository 才具備轉為 Public Preview 的條件。

實作結果：Beatforge 根目錄新增 MIT `LICENSE`，Rust package metadata 同步作者、授權與 repository；Tauri bundle 會把 `LICENSE`、`THIRD_PARTY_NOTICES.md` 與固定版本的 `dependency-inventory.json` 放入 `legal/`。`scripts/Update-ThirdPartyInventory.ps1` 以 Cargo metadata、pnpm production license report、hash-pinned Python requirements 與人工複核 mapping 產生無時間戳及本機路徑的 deterministic inventory，目前覆蓋 Rust、npm、Python 及七個 managed components。Mapperatorinator、slider、模型、CPython、PyTorch／torchaudio 與 imageio-ffmpeg／FFmpeg 明確列為首次設定下載，禁止混入 installer；模型因 training-data provenance 說明不足不由 Beatforge mirror，FFmpeg 則必須在每個 release candidate 依實際 build configuration 人工確認 LGPL／GPL 義務。`scripts/Test-ThirdPartyLicenses.ps1` 會阻擋 lockfile 漂移、unknown license、pin 不一致、notice 缺失及 download-only 元件誤入 bundle，並已納入 release gate；公開揭露、WIP 建議、osu! Ranked AI policy、輸入媒體權利與非官方關係記錄於 `docs/legal/licensing-and-distribution.md`。自動掃描不取代發布前人工審查。

#### 9.5 Windows Installer 與簽章

- 建立正式 Windows installer，定義支援的安裝範圍、捷徑、應用程式資料位置、WebView2／GPU 前置條件與安裝失敗復原。
- 將大型 runtime 與模型維持在受管理資料目錄，由應用程式驗證後下載或修復，避免每次更新重複打包。
- 建立 code signing 流程，對 installer、主程式與 updater artifact 簽章並保存 checksum、版本及 build provenance。
- 測試安裝、重新安裝與解除安裝；解除安裝不得在未明確告知時刪除使用者模型、設定、歷史與生成成果。

驗收：全新支援的 Windows 環境可只透過 installer 啟動 Beatforge 並完成首次設定；checksum 與 build provenance 可驗證，失敗安裝可安全重試。正式 CA 憑證簽章、Publisher 與 SmartScreen 驗收延至 9.8 release candidate，不阻擋後續開發。

實作結果：Windows release 固定使用 x64、current-user NSIS installer，主程式命名為 `Beatforge.exe`，禁止 silent downgrade，內含 Visual C++ runtime，缺少 WebView2 時使用 bootstrapper 補齊。`scripts/Build-WindowsInstaller.ps1` 預設要求乾淨 Git revision、GPU-free release gate、有效 Code Signing certificate／timestamp 與兩個有效 Authenticode signatures；只有明確指定 `-AllowUnsigned` 才能產生不可發布的本機 smoke build。輸出會附 installer、standalone executable、LICENSE、third-party notices、dependency inventory、SHA-256 checksums 與不含私人路徑的 build provenance。`scripts/Test-WindowsInstaller.ps1` 已納入 release gate；`scripts/Test-WindowsInstallerLifecycle.ps1` 只允許在明確確認的 disposable Windows 帳號／VM 執行 install、首次啟動、reinstall、uninstall 及 user-data preservation 測試。完整建置、資料位置、失敗重試與乾淨環境驗收步驟見 `docs/release/windows-installer.md`。已產生並驗證 unsigned NSIS smoke artifact；正式 CA 憑證簽章及 Publisher／SmartScreen 驗收延至 9.8，不阻擋 9.6 與 9.7。

#### 9.6 更新、降級、離線啟動與修復

- 建立簽章驗證的更新 manifest、下載、套用、重新啟動與失敗回滾流程；更新前記錄目前版本與必要 migration 狀態。
- 驗證跨支援版本的 app、settings 與 jobs database migration，確保更新不刪除模型、設定、歷史、raw／validated 成果或匯入快取。
- 定義可支援的降級範圍；遇到較新 schema 時以唯讀／可理解錯誤停止，不得由舊版直接破壞資料。
- 無網路時仍可使用已安裝 runtime／model 完成生成；缺少或損壞的元件則提供可重試、可驗證的修復流程。

驗收：正常更新、更新中斷、簽章錯誤、離線啟動、支援範圍內降級與 runtime／model 修復案例通過，使用者資料保持一致。

實作結果：加入 Tauri updater 與 Settings「立即檢查／安裝並重新啟動」介面，固定使用 HTTPS static manifest、內嵌 public key、`requireSignedVersion` 及 strictly-newer SemVer policy；updater minisign 與 Windows Authenticode 分離，因此 unsigned 開發 installer 仍會產生不可竄改且綁定版本的 `.sig` 與 `latest.json`。套用前會在 app-data 以可回復 replace 寫入 `update-state.json`，下次啟動辨識成功版本或保留舊版並提供安全重試。資料庫在任何 migration／manifest sync 前檢查最高 schema，舊版遇到較新資料會在零寫入下停止並顯示復原說明；settings 與 package manifest 也持續拒絕未知新 schema。更新服務離線只回報 `offline`，不阻擋使用已驗證的本機 runtime／model；下載支援 `.part` resume、size／SHA-256 驗證、有效 cache 離線重用，安裝則透過 staging／backup 防止損壞既有環境。`scripts/Test-UpdatePolicy.ps1` 已加入 release gate，並以單元測試覆蓋 downgrade、future-schema 零寫入、更新狀態回復、cache offline reuse、安裝中斷復原、簽章竄改及 signed-version mismatch。流程與開發 key／正式 release key 邊界記錄於 `docs/release/updates-and-recovery.md`；真正由公開 endpoint 進行的雙版本更新及正式 CA 簽章實機紀錄延至 9.8 release candidate。

#### 9.7 GPU 與驅動支援矩陣

- 依實際可取得設備測試不同 NVIDIA GPU 世代、顯存容量、驅動版本、bf16 支援與常見 Windows 組態。
- 對每個組合記錄能否安裝、模型載入、峰值 VRAM、單難度／多難度生成時間、穩定性與已知限制。
- 對不支援、顯存不足或驅動過舊的環境提供明確的預檢結果與改善建議，不在推論中途才以不明錯誤失敗。
- 將最低需求、推薦規格與僅部分驗證的組合分開標示，避免把未測試硬體宣稱為已支援。

驗收：完成至少最低規格、主要開發／推薦規格與一組低顯存情境的實機紀錄；UI 診斷結果與發布文件中的支援矩陣一致。

#### 9.8 Release Pipeline 與候選版驗收

- 建立從乾淨 source revision 產生前端、Rust、worker resources、installer、checksums、notices 與 release notes 的版本化流程。
- 發布流程執行測試、lint、type-check、Clippy、依賴／授權檢查、artifact smoke test 與簽章驗證，任何必要步驟失敗都停止發布。
- 建立 release notes、已知問題、支援矩陣、升級／修復說明與 rollback 操作手冊。
- 使用 release candidate 在全新 Windows 帳號或機器完成安裝、首次設定、生成、validation、預覽、匯入、更新及診斷匯出的最終實機驗收。

驗收：相同 source revision 的發布內容可追蹤且可重建；release candidate 在乾淨環境完成核心流程後，才允許標記正式版本。

交付物：

- Windows installer
- Release notes
- Third-party notices
- 已知問題與支援矩陣

驗收條件：

- 全新支援環境可由 installer 完成安裝並生成測試譜面。
- 應用程式更新不會刪除模型、設定或歷史。
- 使用者可在不暴露歌曲內容的情況下匯出診斷資訊。

## 建議專案結構

```text
.
├─ README.md
├─ docs/
│  ├─ architecture/
│  ├─ protocol/
│  └─ ui/
│     ├─ osu-ai-desktop-wireframes.drawio
│     └─ exports/
├─ src/                         # Vue 3 frontend
│  ├─ components/
│  ├─ views/
│  ├─ stores/
│  ├─ services/
│  ├─ router/
│  └─ types/
├─ src-tauri/                   # Rust desktop core
│  ├─ src/
│  ├─ capabilities/
│  └─ migrations/
├─ worker/                      # Python AI worker
│  ├─ adapters/
│  ├─ protocol/
│  ├─ validation/
│  └─ tests/
├─ scripts/
└─ tests/
   ├─ fixtures/
   └─ e2e/
```

此結構是規劃方向，應在各階段開始前依實際工具鏈確認，不需要一次建立所有空目錄。

## 安全與隱私原則

- 音訊和 beatmap 預設不離開本機。
- 不將使用者輸入直接拼接成 shell command。
- 所有路徑、IPC request 和下載 manifest 必須驗證。
- Worker 使用獨立工作目錄，避免不同任務互相覆蓋。
- 下載的 runtime 和模型必須驗證 checksum。
- 取消或失敗後，以可追蹤方式清理暫存資料。
- Telemetry 和 crash report 必須透明且可選擇關閉。
- 診斷匯出預設排除歌曲、beatmap 內容和個人路徑等敏感資料。
- 對外的 `.osu` metadata 不得包含模型路徑、使用者名稱或生成參數；可重現資訊只保存在受管理的 provenance。

## 授權與命名注意事項

- Beatforge 原始碼採用 [MIT License](LICENSE)；此授權不涵蓋第三方程式、模型、媒體或使用者內容。
- 完整套件清單、下載／散布邊界與 attribution 見 [Third-party notices](THIRD_PARTY_NOTICES.md) 及 [授權與散布政策](docs/legal/licensing-and-distribution.md)。
- Mapperatorinator、模型、Python runtime 與 FFmpeg 等 AI 環境元件由首次設定下載，不包含在 Beatforge installer；模型 training-data provenance 與每個 FFmpeg build 仍須在正式 release 前人工複核。
- 本專案與 osu! 官方沒有從屬或背書關係。
- 正式發布前需確認專案名稱、圖示和視覺資產不會造成官方產品混淆。
- 使用者應對輸入音訊與輸出內容的使用權負責。

## 後續方向

完成 MVP 並取得真實生成時間、顯存需求與玩家回饋後，再評估：

- taiko、catch、mania 的完整流程。
- 局部 remap、timing-only、hitsound-only 與 guest difficulty。
- MaiMod 品質建議。
- 輕量 beatmap 視覺預覽。
- 擴充生成頁的譜面設定：每個難度可調 HP、OD、基本滑條速度與滑條點倍率；歌曲層級可編輯譜師、原文標題／歌手、來源與公開標籤，並另行評估堆疊寬容度、倒數、寬螢幕及其他 osu! metadata 選項。
- LoRA 與自訂 checkpoint 管理。
- AMD／Linux 支援。
- 可選的遠端 GPU worker，但仍沿用相同 job protocol。

