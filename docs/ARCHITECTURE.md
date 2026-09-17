# Planner 技术架构

## 1. 产品边界

Planner 是 Windows 本地优先桌面应用。发布版由 Tauri 2 承载 React 静态资源，不依赖浏览器、开发服务器、Python 或云服务。所有核心数据均写入用户可控的数据根目录。

默认数据根目录：`%LOCALAPPDATA%/com.planner.desktop/PlannerData/`。用户可在设置中迁移到其他目录；数据库只保存根目录内的相对路径，避免盘符变化破坏关联。

```text
PlannerData/
├── database/planner.db
├── notes/
├── daily/YYYY/MM/YYYY-MM-DD.md
├── assets/images/
├── assets/attachments/
├── backups/YYYY-MM-DD-HHmmss/
└── config/
```

## 2. 分层

- React + TypeScript：界面、路由、交互与客户端状态；不直接拼接文件路径或执行 SQL。
- Tauri commands：稳定、类型化的前后端协议。
- Rust application services：任务、计划、笔记、打卡、统计、搜索、备份等用例及事务边界。
- Rust repositories：`rusqlite` 访问 SQLite；WAL、外键和 busy timeout 默认开启。
- Rust storage：Markdown/图片/附件的路径校验、安全写入、移动与回收站。

## 3. SQLite 设计原则

- 主键使用 UUID 文本，时间统一存 UTC ISO-8601；业务日期存 `YYYY-MM-DD`。
- 所有用户实体支持 `deleted_at` 软删除；回收站恢复时保留原主键与关系。
- 排序字段使用整数 `sort_order`；批量拖拽在单事务内更新。
- 迁移记录保存在 `schema_migrations`，启动时顺序执行。
- `PRAGMA foreign_keys=ON`、`journal_mode=WAL`、`synchronous=FULL`、`busy_timeout=5000`。

完整首版 Schema 位于 `src-tauri/migrations/001_initial.sql`。

## 4. Markdown 与数据库同步

SQLite 是元数据与关系索引，Markdown 文件是正文真源（source of truth）。`documents.relative_path` 始终相对 `PlannerData`。

保存流程：

1. 校验目标路径位于数据根目录，拒绝 `..`、绝对路径和符号链接逃逸。
2. 在同一目录写入唯一临时文件，flush 后执行 `sync_all`。
3. 计算 SHA-256 和文件大小。
4. 将临时文件原子替换目标 `.md`。
5. 在 SQLite 事务中更新哈希、mtime、标题与索引；失败则记录 `sync_state=needs_reindex`，下次启动修复。

重命名/移动流程使用 reservation journal：先在 `file_operations` 记录意图，再移动文件并更新数据库；崩溃恢复器依据 journal 完成或回滚。外部编辑通过 mtime + hash 检测，文件内容优先，重新解析标题与全文索引。数据库记录存在但文件丢失时不静默覆盖，标记 `missing` 并向用户提供恢复/重新定位。

图片粘贴使用日期 + UUID 文件名写入 `assets/images/`，成功落盘后才向编辑器插入相对 Markdown 链接。

## 5. 页面结构

```text
App Shell
├── Dashboard（默认）
├── 今日计划
├── 任务
├── 每日打卡
├── 学习计划 / 计划详情 / 阶段详情
├── 知识库 / Markdown 编辑器（编辑、预览、分屏、沉浸）
├── 日历 / 日期详情
├── 数据统计
├── 回收站
└── 设置（数据目录、主题、自动备份、保留数）
```

全局层包含 Ctrl+K 搜索、Ctrl+N 新任务、Ctrl+Shift+N 新笔记、Ctrl+S 保存、错误提示和未保存状态。

## 6. 状态与安全

- 数据修改由 Rust transaction/use-case 完成，React 不做跨表一致性协调。
- 删除默认软删除；物理清理由回收站显式执行。
- 每次结构迁移前自动备份数据库；手动/周期备份写入新目录，完成后原子写入 manifest。
- 窗口位置、尺寸、最后路由、主题等保存在 `settings`；异常退出后仍可恢复。
- Content Security Policy 在进入 Release 前收紧；前端不加载远程脚本或远程字体。

## 7. Phase 1-10 路线与验收

1. **桌面基础**：Tauri 2 + React + TypeScript + Tailwind；窗口、Sidebar、路由、SQLite 启动迁移、数据目录。验收：TS build、Rust check、启动窗口、重启后 DB 保留。
2. **任务系统**：CRUD、状态、排序、筛选、搜索、软删除、完成与学习记录。验收：事务测试与完整交互。
3. **计划系统**：计划/阶段/任务关联及进度派生。验收：阶段与总进度一致。
4. **知识库目录**：树、文件夹/文档、移动排序、回收站。验收：重启关系不丢失。
5. **Markdown 编辑器**：编辑/预览/分屏、GFM、数学公式、TOC、自动保存。验收：快捷键与恢复。
6. **本地文件存储**：安全写入、图片粘贴/拖入、外部变更检测、迁移数据根目录。验收：故障注入不损坏正文。
7. **打卡与 Daily Note**：学习日志、心情、连续天数、模板。验收：跨日与时区边界。
8. **日历与统计**：月视图、趋势图、占比、年度热力图。验收：与原始记录核对。
9. **搜索与快捷键**：SQLite FTS5、全局命令面板、页面跳转。验收：中英文正文和标签查询。
10. **备份与 Release**：自动/手动备份、保留策略、恢复验证、NSIS/MSI、快捷方式、图标、Release smoke test。

每个 Phase 必须依次通过：前端 lint/typecheck/build、Rust fmt/clippy/test/check、数据库迁移/约束检查、运行时 Console 检查和人工核心流程 smoke test。

## 8. 关键选型

- Tauri 2.x；Windows 使用系统 WebView2，离线安装包阶段评估 `offlineInstaller`。
- React 19、TypeScript、Vite、Tailwind CSS 4、shadcn/ui 组件模式、Lucide。
- SQLite + `rusqlite`（Rust 后端独占写入），FTS5 用于全文检索。
- Markdown 编辑器阶段采用成熟的 CodeMirror 6 生态；预览使用 unified/remark/rehype + KaTeX。
- 图表阶段采用 Recharts；年度热力图自绘语义化网格，减少依赖与提升可访问性。
