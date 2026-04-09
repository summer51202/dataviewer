# DataViewer Software Stack

## 1. 選型原則

這份 stack 建議不是追求「最新最多」，而是追求：

- 單人桌面工具可維護
- 本機檔案與 SQLite 整合順手
- UI 開發速度夠快
- 後續擴充 parser / exporter / CVAT integration 成本低

其中部分選型有參考官方文件確認相容性與採用方式；部分則是依照這個產品的形狀做工程判斷。

## 2. 建議總表

| Layer | 建議選型 | 理由 |
| --- | --- | --- |
| Desktop Shell | Tauri 2 | 適合本機桌面工具，體積小，能用 Rust 處理本機系統能力 |
| Frontend App | React + TypeScript | 元件生態成熟，適合做資料工具與複雜狀態畫面 |
| Build Tool | Vite | 啟動快、開發體驗好，與 React 搭配成熟 |
| UI Styling | Tailwind CSS | 建立桌面工具型 UI 很快，易於建立狀態色與密集版面 |
| UI Primitives | Radix UI | Dialog、Select、Tabs、Tooltip 這類互動元件比較穩 |
| Component Base | shadcn/ui | 可加速建出工具型介面，但元件仍可持有在專案內 |
| Routing | React Router | MVP 頁面不複雜，用成熟方案即可 |
| Server State | TanStack Query | 適合管理 Tauri command 回傳資料與背景 job 狀態 |
| Client State | Zustand | 輕量，適合選取狀態、篩選條件、當前 task 狀態 |
| Desktop Backend | Rust | 很適合做檔案掃描、格式轉換、匯出與本機整合 |
| Local Database | SQLite | 單機工具最合適，無需額外服務 |
| Rust DB Access | rusqlite | 對單機 SQLite 很直接，成本比完整 ORM 低 |
| Serialization | serde | 前後端資料結構序列化標準選擇 |
| File Traversal | walkdir | 做遞迴掃描來源資料夾很適合 |
| Image Metadata | image crate | 讀尺寸、生成必要縮圖時夠用 |
| Annotation Tool | Local CVAT | 已確認支援 COCO / YOLO 匯入匯出與 API 整合 |
| Logging | tracing | Rust 端追查掃描、同步、匯出流程很好用 |
| Testing | Vitest + Rust tests | 前端單元測試與 Rust service 測試分開做 |

## 3. 為什麼不是 Electron

這是工程判斷，不是 Electron 不能做。

對你目前的情境，`Tauri 2` 更適合：

- 單人本機工具，不需要 Chromium 全包
- 需要碰 filesystem、SQLite、匯出流程，Rust side 很有優勢
- 安裝包與常駐記憶體通常會比 Electron 輕

如果未來要大量使用純 Node.js 生態、現成 Electron 插件，Electron 才會比較有優勢。

## 4. 為什麼前端仍然用 React

原因：

- 需要的畫面雖然不是行銷頁，但互動狀態不少
- 有 Sources、Import Review、Browser、CVAT Tasks、Export 等多頁面
- 縮圖牆、篩選器、彈窗、摘要卡片都適合元件化

`React + TypeScript` 可以讓畫面模組和 domain model 比較好對齊。

## 5. 為什麼 UI 層建議 Tailwind + Radix + shadcn/ui

### Tailwind CSS

適合原因：

- 能快速做高密度工具介面
- 類別篩選、摘要卡、狀態框線等調整很快
- 在桌面工具中常需要大量 layout 微調，utility class 會很順手

### Radix UI

適合原因：

- Dialog、Dropdown、Tabs、Select、Tooltip 這些基本互動元件夠穩
- 可保留自己的視覺設計，不會被重型組件庫綁死

### shadcn/ui

適合原因：

- 可以快速落地 Button、Table、Dialog、Form 這些常見元件
- 元件是專案內代碼，不是黑盒套件，之後容易改造
- 對你這種內部工具類產品，開發效率收益很高

## 6. 狀態管理建議

### Zustand 負責

- 目前 workspace 基本狀態
- browser 篩選條件
- 目前已選圖片 id
- UI modal / panel 狀態

### TanStack Query 負責

- source list
- import review payload
- CVAT task list
- annotation version list
- export preview
- 長任務重新整理與快取失效

這樣可以把「本地 UI 狀態」和「來自 Tauri backend 的資料」分開。

## 7. Routing 建議

MVP 頁面雖然不少，但路由複雜度不高，所以建議先用 `React Router`。

建議頁面：

- `/`
- `/workspace/:workspaceId/browser`
- `/workspace/:workspaceId/sources`
- `/workspace/:workspaceId/import-review`
- `/workspace/:workspaceId/cvat`
- `/workspace/:workspaceId/versions`
- `/workspace/:workspaceId/export`
- `/workspace/:workspaceId/image/:imageId`

如果後期希望更強的型別化 search params，再考慮 TanStack Router。

## 8. Rust Core 建議

Rust 端應該被視為「本機 application core」，而不是只做一些零碎 command。

Rust 核心適合承擔：

- source scanning
- COCO / YOLO parsing
- annotation normalization
- SQLite persistence
- health check
- split generation
- export writing
- CVAT API integration

這些工作放在 Rust 會比前端直接做更穩，也更接近資料與檔案。

## 9. SQLite 建議

這是工程推論，不是來自單一官方來源。

採用 SQLite 的理由：

- 單機 embedded database 非常適合你的使用方式
- 不用再安裝 PostgreSQL 或 MySQL
- workspace 可跟著 SQLite 檔一起備份
- 搭配 read-only source data 模式很自然

`rusqlite` 的理由：

- 針對單機 SQLite 很直接
- 不需要為了 MVP 先引入較重的 ORM
- schema 與 query 可明確掌控

## 10. CVAT 整合建議

第一版應採用：

- 本機安裝 CVAT
- DataViewer 維護 temp workspace folder
- 以 API 建立 task / project
- 透過 sync back 建立 annotation version

第一版不建議：

- 直接把 CVAT 內嵌成主要 UI
- 把原始來源資料夾直接掛給 CVAT 使用

原因：

- Windows 路徑、WSL2、Docker 掛載對齊會更複雜
- temp copy 流程比較穩，也比較符合 read-only source 原則

## 11. 開發工具建議

| 類型 | 建議 |
| --- | --- |
| Package Manager | pnpm |
| Lint | ESLint |
| Format | Prettier |
| Type Check | TypeScript strict mode |
| Frontend Test | Vitest + Testing Library |
| Rust Test | cargo test |
| Bundling | Tauri bundler |

備註：

- `pnpm` 是工程建議，不是強制
- 若團隊之後要更穩定的 monorepo 體驗，也可再評估 workspace 管理

## 12. 版本與採用策略

建議採用原則：

- 使用目前穩定主版本
- 不在 MVP 初期追求過多 beta 生態
- 先讓 Tauri、React、Vite、CVAT 走穩定搭配

對某些套件的判斷：

- `Tauri 2`：建議直接上 v2，不建議新專案從 v1 起步
- `React`：使用目前主流穩定版搭配 TypeScript
- `CVAT`：用官方文件對應的 Docker / WSL2 安裝流程

## 13. 最終推薦組合

如果要收斂成一套最小但夠用的 MVP stack，我會建議：

- Tauri 2
- React + TypeScript
- Vite
- Tailwind CSS
- Radix UI
- shadcn/ui
- React Router
- Zustand
- TanStack Query
- Rust
- SQLite + rusqlite
- Local CVAT

## 14. 參考來源

以下為本次選型時參考的官方文件：

- Tauri Docs: https://v2.tauri.app/start/
- Tauri Create Project: https://v2.tauri.app/start/create-project/
- React Docs: https://react.dev/learn/react-compiler/installation
- Vite Docs: https://vite.dev/guide/
- Tailwind CSS with Vite: https://v3.tailwindcss.com/docs/guides/vite
- Radix UI Docs: https://www.radix-ui.com/primitives/docs/overview/introduction
- shadcn/ui Docs: https://ui.shadcn.com/docs
- shadcn/ui Installation: https://ui.shadcn.com/docs/installation
- CVAT Installation: https://docs.cvat.ai/docs/administration/community/basics/installation
- CVAT API: https://docs.cvat.ai/docs/api_sdk/api/
- CVAT COCO Format: https://docs.cvat.ai/docs/dataset_management/formats/format-coco/
- CVAT YOLO Format: https://docs.cvat.ai/docs/dataset_management/formats/format-yolo/
