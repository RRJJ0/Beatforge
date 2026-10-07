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

正式 bootstrap 以 embedded resource 隨應用程式發布，啟動前會原子部署到 application-managed `runtime/worker/2/beatforge_worker.py`，並由私有 Python 的 isolated mode 執行。worker stdin／stdout 只接受 protocol v1 的單行 JSON，單則訊息上限 64 KiB；upstream import、traceback 與其他診斷一律重新導向 stderr，Rust 最多保存 256 KiB 至 `logs/production-worker.log`，避免污染 JSON Lines channel。

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

worker 會回報 `adapter_ready`、`loading_runtime`、`configuring`、`loading_model`、`preparing_audio`、`generating` 與 `validating_output` 等穩定階段。模型載入和長時間生成期間每 15 秒送出 JSONL heartbeat，Rust 對 production worker 使用一小時無回報 timeout；完成後只接受 managed job directory 內的 `.osu`／`.osz`。音訊解碼固定使用 `imageio-ffmpeg` 診斷出的絕對路徑；由於該 wheel 不附 `ffprobe`，adapter 會略過 pydub 非必要的 probe metadata，仍由 FFmpeg 自動選擇 decoder 與 PCM 輸出。固定版自訂 model config 原本會重新查詢 `openai/whisper-small`；adapter 改用 manifest 已驗證的 V32 `config.json` 內嵌 `backbone_config`，backbone identity 或結構不符便中止，不提供網路 fallback。V32 的實驗性 CUDA graph fast decoder 在 PyTorch 2.10 timing pass 可能把全零機率分布傳給 `torch.multinomial` 並觸發 device-side assert，因此 MVP 固定關閉 `fast_decoder_loop`／`super_timing_fast_loop`，改走上游標準 generation path。V32 偶爾會在序列結尾產生只有 object type／屬性、卻在 EOS 前缺少 `TIME_SHIFT` 的殘缺 type-first group；上游 postprocessor 會把 dataclass 的預設 `0 ms` 誤當成真實時間。adapter 只丟棄這種無法定位的殘缺 group，並把數量寫入 warning 與 provenance，不會移動、排序或猜測任何有時間 token 的物件。第一版設定 `export_osz=true`，將音訊與生成譜面封裝成可匯入 osu! 的 `.osz`；reference `.osu` 使用 timing context。完整輸出內容與語意驗證仍屬階段 6。

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
- [ ] 6.8：完成 osu! 開啟／匯入流程與實體驗收（程式與自動測試完成，待實機驗收）。

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

Jobs UI 對 completed 且具有 validated artifact 的任務提供 **Show validated output** 與 **Import into osu!**。每次操作都會由 Rust 重新讀取 SQLite，要求 job 為 completed、report 可交付、report job identity 一致，並比對 stored output、`validated/<filename>` relative path 與 canonical filesystem path；symlink、`raw`／`working`、外部路徑、非 `.osu`／`.osz` 或已遺失檔案都會拒絕，不信任前端傳入實體路徑。

Windows 匯入會優先使用 Settings 明確選擇且檔名為 `osu!.exe` 的執行檔，其次使用偵測到的 stable／lazer executable。手動匯入在找不到 executable 時可交給 Windows `.osu`／`.osz` file association；自動匯入只接受已驗證 executable，不使用 association fallback，並且只在完整 validation 成功後啟動。實機 stable／lazer 驗收清單位於 `docs/testing/stage-6-physical-acceptance.md`；完成該清單前 6.8 維持待驗收狀態。

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

- [ ] 7.1：建立正式歷史資料檢視、搜尋、篩選與排序。
- [ ] 7.2：完成歷史任務刪除、重新生成與檔案一致性處理。
- [ ] 7.3：建立設定預設值、版本化 migration 與匯入／匯出。
- [ ] 7.4：顯示磁碟用量並提供安全的 cache／暫存資料清理。
- [ ] 7.5：加入桌面通知與背景任務完成／失敗提示。
- [ ] 7.6：加入輕量音訊、時間軸與物件密度預覽。
- [ ] 7.7：完成繁體中文／英文介面與 accessibility 改善。
- [ ] 7.8：執行產品整合、migration 與日常流程驗收。

#### 7.1 歷史檢視、搜尋與篩選

- 將 History 畫面改為讀取 SQLite 的正式 job records，不使用展示資料。
- 支援關鍵字、狀態、模型與日期篩選，以及建立時間排序。
- 明確呈現完成、驗證失敗、推論失敗、取消與中斷狀態。
- 處理大量歷史紀錄，避免一次渲染造成介面卡頓。

驗收：重新啟動後仍可查到既有任務；組合篩選與空結果狀態正確，畫面內容與 SQLite 一致。

#### 7.2 歷史動作與檔案一致性

- 刪除前顯示影響範圍，並安全移除 job record 與該 job 的受管理檔案。
- 重新生成時複製原任務可重用的輸入與有效設定，建立新的 job identity，不覆寫舊任務。
- 輸入音訊、reference 或輸出檔遺失時顯示可理解狀態與可行動提示。
- 加入刪除、重新生成及資料庫／檔案不同步的測試。

驗收：操作只影響指定 job；無法解析或位於受管理目錄外的路徑不得刪除，舊任務與 provenance 保持可追蹤。

#### 7.3 設定預設值、匯入／匯出與 migration

- 定義 versioned settings schema、平台預設值、欄位驗證與向後相容 migration。
- 提供 JSON 設定匯出與匯入預覽；匯入前列出將變更、忽略或重設的欄位。
- 敏感、機器限定或不存在的路徑不盲目套用，匯入失敗不破壞目前設定。
- 提供單項或全部恢復預設值，並測試舊版本設定升級。

驗收：設定可跨重啟保存；合法設定可往返匯出／匯入，損壞或較新版本檔案會被安全拒絕。

#### 7.4 磁碟用量與安全清理

- 統計 runtime、model、cache、logs、jobs raw／working／validated 的用量與最後更新時間。
- 提供可預覽的清理選項，區分可重新下載 cache、暫存資料、舊 logs 與使用者成果。
- 清理採 managed-root allowlist，拒絕 symlink、越界路徑、執行中 job 與預設刪除 validated 成果。
- 清理完成後重新計算用量並回報釋放空間與失敗項目。

驗收：預覽數量與實際清理結果一致；清理不影響已安裝模型、執行中任務或使用者未明確選擇的成果。

#### 7.5 桌面通知與背景狀態

- 在任務完成、驗證失敗或推論失敗時提供可設定的 Windows 桌面通知。
- 應用程式在前景時避免重複干擾；背景通知可帶回對應任務畫面。
- 通知只顯示必要摘要，不包含 traceback、完整本機路徑或其他敏感資料。
- 通知權限不可用時降級為應用程式內提示，不影響任務結果。

驗收：各終止狀態至多通知一次；關閉通知後不再顯示系統通知，重新啟動不重播舊通知。

#### 7.6 輕量結果預覽

- 提供本機音訊播放、播放位置與基本波形／時間軸。
- 由已驗證的 statistics 或 typed beatmap 資料產生物件密度摘要，支援多 difficulty 切換。
- 預覽資料必須來自 validated artifact，並限制解析大小與 UI 點數，避免阻塞主介面。
- 此功能只用於檢視與定位，不提供 hit object 編輯或取代 osu! Editor。

驗收：播放、seek、difficulty 切換與密度顯示一致；遺失或不支援的音訊會安全降級且不影響匯入。

#### 7.7 雙語與 Accessibility

- 將使用者可見字串集中管理，完成繁體中文與英文切換及保存。
- 檢查鍵盤導覽、focus 順序、focus 樣式、dialog focus trap、表單 label 與錯誤提示。
- 改善文字／狀態對比、縮放、長字串、減少動態效果與不只依賴顏色傳達狀態。
- 為主要流程建立 accessibility checklist，修正自動檢查與人工鍵盤測試發現的問題。

驗收：不使用滑鼠即可完成設定、建立任務、查看結果與匯入；兩種語言下主要畫面無截斷或未翻譯字串。

#### 7.8 產品整合與日常流程驗收

- 串接 History、Settings、storage、notifications、preview 與既有生成／驗證／匯入流程。
- 測試由舊資料庫與舊設定升級、重啟恢復、遺失檔案、磁碟不足與清理後狀態。
- 在一般 Windows 使用者環境完成不查看命令列的日常流程測試。
- 記錄已知限制、驗收結果與階段 8 發布前仍需處理的項目。

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

### 階段 8：測試、封裝與發布

目標：建立可重現、可更新和可支援的 Windows 發布流程。

工作項目：

- 建立單元、整合、端到端與長時間穩定性測試。
- 測試不同 NVIDIA GPU、顯存與驅動版本。
- 測試更新、降級、離線啟動和修復流程。
- 建立簽章、installer 與自動更新。
- 建立授權清單、第三方 notices 與隱私說明。
- 檢查 Mapperatorinator 程式、權重、資料與依賴授權。
- 建立 issue template 與診斷資料回報流程。

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

## 授權與命名注意事項

- Mapperatorinator 程式碼、模型權重、訓練資料與第三方依賴需分別確認授權。
- 本專案與 osu! 官方沒有從屬或背書關係。
- 正式發布前需確認專案名稱、圖示和視覺資產不會造成官方產品混淆。
- 使用者應對輸入音訊與輸出內容的使用權負責。

## 後續方向

完成 MVP 並取得真實生成時間、顯存需求與玩家回饋後，再評估：

- taiko、catch、mania 的完整流程。
- 局部 remap、timing-only、hitsound-only 與 guest difficulty。
- MaiMod 品質建議。
- 輕量 beatmap 視覺預覽。
- LoRA 與自訂 checkpoint 管理。
- AMD／Linux 支援。
- 可選的遠端 GPU worker，但仍沿用相同 job protocol。

