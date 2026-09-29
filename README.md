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

### 階段 5：Mapperatorinator 整合

目標：以正式 Python worker 完成第一個端到端 beatmap 生成。

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

