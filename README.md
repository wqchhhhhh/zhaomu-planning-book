# 朝暮

> zhaomu-planning-book

本地优先的个人任务、项目规划、学习打卡和文档知识库桌面软件。应用名称寓意“朝有计划，暮有所得”。

## 功能

- 支持周期、时间跨度、截止日期和重点标记的任务管理
- 任务每日安排、今日计划、完成进度和启动提醒
- 项目规划及项目任务
- 两级分类的本地文档知识库
- 每日打卡、Daily Note、日历、学习统计和全局搜索
- SQLite 本地数据库、软删除、自动备份和深色模式

知识库上传文件默认保存在 `E:\朝暮数据\knowledge`。数据库及个人数据不会包含在源码仓库中。

## 技术栈

Tauri 2、React、TypeScript、Vite、Rust、SQLite、Tailwind CSS、Recharts、KaTeX。

## 开发命令

```powershell
pnpm install
pnpm check
pnpm tauri dev
```

Windows 原生开发需要 Rust stable MSVC、Microsoft C++ Build Tools（Desktop development with C++）以及 WebView2。发布构建使用：

```powershell
pnpm tauri build
```

发布版数据默认存放在系统推荐的应用本地数据目录下的 `PlannerData`，而不是安装目录。

详细架构参见 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)。
