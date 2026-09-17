import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import "katex/dist/katex.min.css";
import {
  ArchiveRestore,
  BarChart3,
  BookOpen,
  CalendarDays,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Circle,
  Clock3,
  FilePlus2,
  Flame,
  FolderKanban,
  FolderPlus,
  Home,
  ListTodo,
  Menu,
  Moon,
  Plus,
  Save,
  Search,
  Settings,
  Sun,
  Trash2,
  X,
} from "lucide-react";
import {
  HashRouter,
  NavLink,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from "react-router-dom";
type Boot = { dataRoot: string; databasePath: string; schemaVersion: number };
type Task = {
  id: string;
  title: string;
  description: string;
  status: "todo" | "doing" | "done";
  priority: string;
  plannedDate: string | null;
  estimatedMinutes: number | null;
  actualMinutes: number;
  planId: string | null;
  projectId: string | null;
  isImportant: boolean;
  dueAt: string | null;
  scheduleStart: string | null;
  scheduleEnd: string | null;
  recurrence: string;
  remindOnOpen: boolean;
};
type Plan = {
  id: string;
  title: string;
  description: string;
  kind: string;
  startDate: string | null;
  endDate: string | null;
  total: number;
  done: number;
};
type Stage = {
  id: string;
  title: string;
  description: string;
  total: number;
  done: number;
};
type Doc = {
  id: string;
  parentId: string | null;
  kind: "folder" | "note";
  title: string;
  relativePath: string;
  updatedAt: string;
};
type Stats = {
  todayMinutes: number;
  weekMinutes: number;
  monthMinutes: number;
  studyDays: number;
  trend: { date: string; minutes: number; tasks: number }[];
};
type Cal = {
  date: string;
  total: number;
  done: number;
  minutes: number;
  checkedIn: boolean;
};
type Hit = {
  type: "task" | "plan" | "note";
  id: string;
  title: string;
  subtitle: string;
};
type Project = {
  id: string;
  title: string;
  description: string;
  status: string;
  startDate: string | null;
  endDate: string | null;
  total: number;
  done: number;
};
type DailyItem = { id: string; taskId: string; taskTitle: string; itemDate: string; content: string; isDone: boolean };
type KnowledgeCategory = { id: string; parentId: string | null; title: string };
type KnowledgeFile = { id: string; categoryId: string; title: string; fileName: string; relativePath: string; fileSize: number };
type TaskProgress = { taskId: string; total: number; done: number };
const call = <T,>(c: string, a: Record<string, unknown> = {}) =>
    invoke<T>(c, a),
  today = () => {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  },
  fmt = (n: number) =>
    n >= 60 ? `${Math.floor(n / 60)}h ${n % 60}min` : `${n}min`;
const nav = [
    ["首页", "/", Home],
    ["今日计划", "/today", CalendarDays],
    ["任务", "/tasks", ListTodo],
    ["每日打卡", "/check-in", Flame],
    ["项目", "/projects", FolderKanban],
    ["知识库", "/knowledge", BookOpen],
    ["日历", "/calendar", CalendarDays],
    ["数据统计", "/analytics", BarChart3],
  ] as const,
  bottom = [
    ["回收站", "/trash", ArchiveRestore],
    ["设置", "/settings", Settings],
  ] as const;
function Shell() {
  const loc = useLocation(),
    go = useNavigate(),
    [side, setSide] = useState(false),
    [theme, setTheme] = useState(
      localStorage.theme ??
        (matchMedia("(prefers-color-scheme:dark)").matches ? "dark" : "light"),
    ),
    [boot, setBoot] = useState<Boot | null>(null),
    [search, setSearch] = useState(false),
    [reminders,setReminders]=useState<string[]>([]),[showReminders,setShowReminders]=useState(true);
  const restored = useRef(false);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.theme = theme;
  }, [theme]);
  useEffect(() => {
    call<Boot>("bootstrap").then(setBoot);
    call<string[]>("open_reminders",{date:today()}).then(setReminders);
  }, []);
  useEffect(() => {
    if (!restored.current) {
      restored.current = true;
      const last = localStorage.lastPath;
      if (last && last !== "/") go(last);
      return;
    }
    localStorage.lastPath = loc.pathname;
  }, [loc.pathname, go]);
  useEffect(() => {
    const k = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setSearch(true);
      }
      if (e.ctrlKey && e.key.toLowerCase() === "n") {
        e.preventDefault();
        go(e.shiftKey ? "/knowledge" : "/tasks");
      }
    };
    addEventListener("keydown", k);
    return () => removeEventListener("keydown", k);
  }, [go]);
  const title =
    [...nav, ...bottom].find((x) => x[1] === loc.pathname)?.[0] ?? "朝暮";
  return (
    <div className="app-shell">
      <aside className={`sidebar ${side ? "sidebar--open" : ""}`}>
        <div className="brand">
          <div className="brand__mark">
            <img src="/chaomu-icon.png" />
          </div>
          <div>
            <strong>朝暮</strong>
            <span>朝有计划，暮有所得</span>
          </div>
          <button
            className="icon-button sidebar__close"
            onClick={() => setSide(false)}
          >
            <X />
          </button>
        </div>
        <NavLink className="quick-add" to="/tasks">
          <Plus />
          新建任务<kbd>Ctrl N</kbd>
        </NavLink>
        <nav className="nav-list">
          {nav.map(([l, p, I]) => (
            <NavLink
              key={p}
              to={p}
              className={({ isActive }) =>
                `nav-item ${isActive ? "nav-item--active" : ""}`
              }
            >
              <I />
              <span>{l}</span>
            </NavLink>
          ))}
        </nav>
        <nav className="nav-list nav-list--bottom">
          {bottom.map(([l, p, I]) => (
            <NavLink
              key={p}
              to={p}
              className={({ isActive }) =>
                `nav-item ${isActive ? "nav-item--active" : ""}`
              }
            >
              <I />
              <span>{l}</span>
            </NavLink>
          ))}
        </nav>
        <div className="local-status">
          <span className="status-dot" />
          <div>
            <strong>本地数据已连接</strong>
            <span>SQLite Schema v{boot?.schemaVersion ?? "…"}</span>
          </div>
        </div>
      </aside>
      {side && (
        <button className="sidebar-backdrop" onClick={() => setSide(false)} />
      )}
      <main className="workspace">
        <header className="topbar">
          <div className="topbar__title">
            <button
              className="icon-button menu-button"
              onClick={() => setSide(true)}
            >
              <Menu />
            </button>
            {title}
          </div>
          <div className="topbar__actions">
            <button className="search-button" onClick={() => setSearch(true)}>
              <Search />
              <span>搜索任务、计划和笔记</span>
              <kbd>Ctrl K</kbd>
            </button>
            <button
              className="icon-button"
              onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            >
              {theme === "dark" ? <Sun /> : <Moon />}
            </button>
          </div>
        </header>
        {showReminders&&reminders.length>0&&<div className="modal-backdrop reminder-backdrop"><section className="reminder-dialog"><div className="reminder-sky"><div className="reminder-orbit"><Sun/><Moon/></div><span>{new Intl.DateTimeFormat("zh-CN",{month:"long",day:"numeric",weekday:"long"}).format(new Date())}</span><button className="reminder-close" aria-label="关闭提醒" onClick={()=>setShowReminders(false)}><X/></button></div><div className="reminder-body"><p className="reminder-kicker">CHAO MU · TODAY</p><h2>{new Date().getHours()<12?"早上好，开启今天的计划":"晚上好，看看今天的收获"}</h2><p className="reminder-quote">朝有计划，暮有所得。今天有 <strong>{reminders.length}</strong> 项值得专注。</p><div className="reminder-list">{reminders.map((x,index)=><div className="reminder-item" key={`${x}-${index}`}><span>{String(index+1).padStart(2,"0")}</span><strong>{x}</strong><i/></div>)}</div><div className="reminder-actions"><button className="reminder-later" onClick={()=>setShowReminders(false)}>稍后再看</button><button className="reminder-open" onClick={()=>{setShowReminders(false);go("/today")}}>开始今天 <ChevronRight/></button></div></div></section></div>}
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/today" element={<Tasks todayOnly />} />
          <Route path="/tasks" element={<Tasks />} />
          <Route path="/check-in" element={<Checkin />} />
          <Route path="/projects" element={<Projects />} />
          <Route path="/knowledge" element={<UploadedKnowledge />} />
          <Route path="/calendar" element={<Calendar />} />
          <Route path="/analytics" element={<Analytics />} />
          <Route path="/trash" element={<Trash />} />
          <Route path="/settings" element={<SettingsPage boot={boot} />} />
        </Routes>
      </main>
      {search && <SearchBox close={() => setSearch(false)} go={go} />}
    </div>
  );
}
function Dashboard() {
  const [tasks, setTasks] = useState<Task[]>([]),
    [dailyItems,setDailyItems]=useState<DailyItem[]>([]),
    [s, setS] = useState<Stats | null>(null);
  useEffect(() => {
    call<Task[]>("list_tasks", { plannedDate: today() }).then(setTasks);
    call<DailyItem[]>("today_daily_items",{date:today()}).then(setDailyItems);
    call<Stats>("analytics").then(setS);
  }, []);
  const scheduledTaskIds=new Set(dailyItems.map(x=>x.taskId)),
    standaloneTasks=tasks.filter(x=>!scheduledTaskIds.has(x.id)),
    done = standaloneTasks.filter((x) => x.status === "done").length+dailyItems.filter(x=>x.isDone).length,
    total=standaloneTasks.length+dailyItems.length,
    p = total ? Math.round((done / total) * 100) : 0,
    h = new Date().getHours(),
    g = h < 12 ? "早上好" : h < 18 ? "下午好" : "晚上好",
    words = [
      "把今天最重要的一件事，认真完成。",
      "每一次专注，都会在暮色里成为收获。",
      "不必追赶所有方向，向前一步就很好。",
      "今日事，今日成；微小积累，自有回响。",
    ];
  return (
    <div className="page dashboard">
      <section className="welcome-row">
        <div>
          <p className="eyebrow">
            {new Intl.DateTimeFormat("zh-CN", { dateStyle: "full" }).format(
              new Date(),
            )}
          </p>
          <h1>{g} 👋</h1>
          <p>{words[new Date().getDate() % 4]}</p>
        </div>
        <NavLink className="primary-button" to="/today">
          <Plus />
          添加今日任务
        </NavLink>
      </section>
      <section className="dashboard-grid">
        <div className="task-section">
          <Head title="今日任务" sub="专注于最重要的三件事" />
          <div className="task-list">
            {dailyItems.map(i=><div className={`task-row ${i.isDone?"task-row--done":""}`} key={`daily-${i.id}`}>{i.isDone?<CheckCircle2 className="task-check--done"/>:<Circle className="task-check"/>}<div className="task-copy"><strong>{i.content}</strong><span>{i.taskTitle} · 今日安排</span></div><span className="today-badge">今日</span></div>)}
            {standaloneTasks.map((t) => (
              <div
                className={`task-row ${t.status === "done" ? "task-row--done" : ""} ${t.isImportant ? "task-row--important" : ""}`}
                key={t.id}
              >
                {t.status === "done" ? (
                  <CheckCircle2 className="task-check--done" />
                ) : (
                  <Circle className="task-check" />
                )}
                <div className="task-copy">
                  <strong>{t.title}</strong>
                  <span>{t.description || "今日任务"}</span>
                </div>
                <span className="task-time">
                  <Clock3 />
                  {t.estimatedMinutes ?? 0} min
                </span>
              </div>
            ))}
            {!total && (
              <p className="empty-copy">
                今天还没有任务，写下最重要的一件事吧。
              </p>
            )}
          </div>
        </div>
        <aside className="today-summary">
          <p className="eyebrow">今日进度</p>
          <div className="progress-number">
            <strong>{done}</strong>
            <span>/ {total}</span>
            <b>{p}%</b>
          </div>
          <Progress n={p} />
          <div className="summary-divider" />
          <Metric
            icon={<Clock3 />}
            label="今日学习"
            value={fmt(s?.todayMinutes ?? 0)}
          />
          <Metric
            icon={<Flame />}
            label="今年学习"
            value={`${s?.studyDays ?? 0} 天`}
          />
        </aside>
      </section>
      <section className="lower-grid">
        <NavLink className="quiet-card action-card" to="/knowledge">
          <BookOpen />
          <div>
            <h2>打开知识库</h2>
            <p>继续书写本地 Markdown 笔记</p>
          </div>
        </NavLink>
        <Daily compact />
      </section>
    </div>
  );
}
function Tasks({ todayOnly = false }: { todayOnly?: boolean }) {
  const [list, setList] = useState<Task[]>([]),
    [title, setTitle] = useState(""),
    [minutes, setMinutes] = useState("45"),
    [filter, setFilter] = useState("all"),
    [record, setRecord] = useState<Task | null>(null),
    [dragged, setDragged] = useState<string | null>(null),
    [projects, setProjects] = useState<Project[]>([]),
    [projectId, setProjectId] = useState(""),
    [important, setImportant] = useState(false),
    [start, setStart] = useState(today()),
    [end, setEnd] = useState(""),
    [due, setDue] = useState(""),
    [recurrence, setRecurrence] = useState("none"),
    [selectedTask, setSelectedTask] = useState<Task | null>(null),
    [dailyItems, setDailyItems] = useState<DailyItem[]>([]),
    [dailyDate, setDailyDate] = useState(today()),
    [dailyContent, setDailyContent] = useState(""),
    [progress,setProgress]=useState<Record<string,TaskProgress>>({});
  const loadProgress=()=>call<TaskProgress[]>("task_daily_progress").then(rows=>setProgress(Object.fromEntries(rows.map(x=>[x.taskId,x]))));
  const load = useCallback(
    () =>
      call<Task[]>("list_tasks", {
        plannedDate: todayOnly ? today() : null,
      }).then(setList),
    [todayOnly],
  );
  useEffect(() => {
    load();
    call<Project[]>("list_projects").then(setProjects);
    void loadProgress();
    if (todayOnly) call<DailyItem[]>("today_daily_items", { date: today() }).then(setDailyItems);
  }, [load]);
  const add = async () => {
    if (!title.trim()) return;
    await call("create_task", {
      input: {
        title,
        plannedDate: todayOnly ? today() : null,
        estimatedMinutes: Number(minutes),
        priority: "medium",
        projectId: projectId || null,
        isImportant: important,
        scheduleStart: start || null,
        scheduleEnd: end || null,
        dueAt: due || null,
        recurrence,
        remindOnOpen: true,
      },
    });
    setTitle("");
    setImportant(false);
    load();
  };
  return (
    <Page
      over={todayOnly ? today() : "全部任务"}
      title={todayOnly ? "今日计划" : "任务管理"}
      sub="任务、状态与学习时间均保存在本地 SQLite。"
    >
      <div className="segmented">
        {["all", "todo", "doing", "done"].map((x) => (
          <button
            key={x}
            className={filter === x ? "active" : ""}
            onClick={() => setFilter(x)}
          >
            {
              (
                {
                  all: "全部",
                  todo: "待办",
                  doing: "进行中",
                  done: "已完成",
                } as Record<string, string>
              )[x]
            }
          </button>
        ))}
      </div>
      <Composer
        title={title}
        setTitle={setTitle}
        minutes={minutes}
        setMinutes={setMinutes}
        add={add}
      />
      <div className="task-links">
        <select
          value={projectId}
          onChange={(e) => {
            setProjectId(e.target.value);
          }}
        >
          <option value="">不加入项目</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.title}
            </option>
          ))}
        </select>
        <label>开始<input type="date" value={start} onChange={e=>setStart(e.target.value)} /></label>
        <label>结束<input type="date" value={end} onChange={e=>setEnd(e.target.value)} /></label>
        <label>截止<input type="date" value={due} onChange={e=>setDue(e.target.value)} /></label>
        <select value={recurrence} onChange={e=>setRecurrence(e.target.value)}>
          <option value="none">单次任务</option><option value="daily">每天</option><option value="weekdays">工作日</option><option value="weekly">每周</option>
        </select>
        <label>
          <input
            type="checkbox"
            checked={important}
            onChange={(e) => setImportant(e.target.checked)}
          />{" "}
          标注为重点任务
        </label>
      </div>
      <div className="managed-list">
        {todayOnly && dailyItems.map(i=><article className={`managed-task ${i.isDone?"managed-task--done":""}`} key={i.id}><button className="task-toggle" onClick={async()=>{await call("set_daily_item_done",{id:i.id,done:!i.isDone});setDailyItems(await call("today_daily_items",{date:today()}))}}>{i.isDone?<CheckCircle2/>:<Circle/>}</button><div><strong>{i.content}</strong><span>来自任务 · {i.taskTitle}</span></div></article>)}
        {list
          .filter((t) => filter === "all" || t.status === filter)
          .map((t) => (
            <article
              draggable
              onDragStart={() => setDragged(t.id)}
              onDragOver={(e) => e.preventDefault()}
              onDrop={async () => {
                if (!dragged || dragged === t.id) return;
                const next = [...list];
                const from = next.findIndex((x) => x.id === dragged);
                const to = next.findIndex((x) => x.id === t.id);
                const [moving] = next.splice(from, 1);
                next.splice(to, 0, moving);
                setList(next);
                await call("reorder_tasks", { ids: next.map((x) => x.id) });
                setDragged(null);
              }}
              className={`managed-task ${t.status === "done" ? "managed-task--done" : ""} ${t.isImportant ? "managed-task--important" : ""}`}
              key={t.id}
            >
              <button
                className="task-toggle"
                onClick={async () => {
                  if (t.status !== "done") return setRecord(t);
                  await call("set_task_status", { id: t.id, status: "todo" });
                  load();
                }}
              >
                {t.status === "done" ? <CheckCircle2 /> : <Circle />}
              </button>
              <div>
                <strong
                  title="双击编辑任务名称"
                  onDoubleClick={async () => {
                    const changed = prompt("编辑任务名称", t.title);
                    if (!changed || changed === t.title) return;
                    await call("update_task", { id: t.id, title: changed });
                    load();
                  }}
                  onClick={async()=>{setSelectedTask(t);setDailyItems(await call("list_task_daily_items",{taskId:t.id}))}}
                >
                  {t.title}
                </strong>
                <span>
                  {t.scheduleStart ?? t.plannedDate ?? "未安排日期"}{t.scheduleEnd ? ` → ${t.scheduleEnd}`:""} · 预计{" "}
                  {t.estimatedMinutes ?? 0} 分钟 · 实际 {t.actualMinutes} 分钟
                </span>
                {(progress[t.id]?.total??0)>0&&<div className="task-progress"><span style={{width:`${Math.round(progress[t.id].done/progress[t.id].total*100)}%`}}/><small>{progress[t.id].done}/{progress[t.id].total} · {Math.round(progress[t.id].done/progress[t.id].total*100)}%</small></div>}
              </div>
              <span className={`priority priority--${t.priority}`}>
                {t.isImportant ? "重点" : t.priority}
              </span>
              <button
                className={`important-button ${t.isImportant ? "active" : ""}`}
                title="切换重点"
                onClick={async () => {
                  await call("set_task_important", {
                    id: t.id,
                    important: !t.isImportant,
                  });
                  load();
                }}
              >
                ★
              </button>
              <button
                className="delete-button"
                onClick={async () => {
                  await call("delete_task", { id: t.id });
                  load();
                }}
              >
                <Trash2 />
              </button>
            </article>
          ))}
      </div>
      {!list.length && <Empty text="这里很清爽，添加一项计划吧。" />}
      {selectedTask && <section className="detail-panel schedule-panel"><Head title={`${selectedTask.title} · 每日安排`} sub="安排每天要学或要做的内容；当天会自动出现在今日计划。"/>{(progress[selectedTask.id]?.total??0)>0&&<div className="schedule-summary"><strong>{Math.round(progress[selectedTask.id].done/progress[selectedTask.id].total*100)}%</strong><div><span>安排进度</span><Progress n={Math.round(progress[selectedTask.id].done/progress[selectedTask.id].total*100)}/></div><small>{progress[selectedTask.id].done} / {progress[selectedTask.id].total}</small></div>}<div className="inline-form compact"><input type="date" value={dailyDate} onChange={e=>setDailyDate(e.target.value)}/><input value={dailyContent} onChange={e=>setDailyContent(e.target.value)} placeholder="当天要完成的内容"/><button onClick={async()=>{if(!dailyContent.trim())return;await call("add_task_daily_item",{taskId:selectedTask.id,itemDate:dailyDate,content:dailyContent});setDailyContent("");setDailyItems(await call("list_task_daily_items",{taskId:selectedTask.id}));loadProgress()}}><Plus/>添加</button></div>{dailyItems.map(i=><div className={`stage-row daily-row ${i.isDone?"done":""}`} key={i.id}><label><input type="checkbox" checked={i.isDone} onChange={async()=>{await call("set_daily_item_done",{id:i.id,done:!i.isDone});setDailyItems(await call("list_task_daily_items",{taskId:selectedTask.id}));loadProgress()}}/><time>{i.itemDate}</time><span>{i.content}</span></label></div>)}</section>}
      {record && (
        <Study
          task={record}
          close={() => setRecord(null)}
          done={() => {
            setRecord(null);
            load();
          }}
        />
      )}
    </Page>
  );
}
function Plans() {
  const [plans, setPlans] = useState<Plan[]>([]),
    [title, setTitle] = useState(""),
    [selected, setSelected] = useState<Plan | null>(null),
    [stages, setStages] = useState<Stage[]>([]),
    [stage, setStage] = useState("");
  const load = () => call<Plan[]>("list_plans").then(setPlans);
  useEffect(() => {
    void load();
  }, []);
  useEffect(() => {
    if (selected)
      call<Stage[]>("list_stages", { planId: selected.id }).then(setStages);
  }, [selected]);
  return (
    <Page
      over="学习与项目"
      title="计划"
      sub="从总目标拆到阶段，再落实为今天的任务。"
    >
      <div className="inline-form">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="新计划名称"
        />
        <button
          onClick={async () => {
            if (!title.trim()) return;
            await call("create_plan", {
              input: { title, kind: "study", startDate: today() },
            });
            setTitle("");
            load();
          }}
        >
          <Plus />
          创建计划
        </button>
      </div>
      <div className="plan-grid">
        {plans.map((p) => {
          const n = p.total ? Math.round((p.done / p.total) * 100) : 0;
          return (
            <button
              key={p.id}
              className={`plan-card ${selected?.id === p.id ? "selected" : ""}`}
              onClick={() => setSelected(p)}
            >
              <span className="plan-kind">学习计划</span>
              <h2>{p.title}</h2>
              <p>
                {p.description ||
                  `${p.startDate ?? "未设置"} → ${p.endDate ?? "长期"}`}
              </p>
              <Progress n={n} />
              <small>
                {p.done}/{p.total} 个任务 · {n}%
              </small>
            </button>
          );
        })}
      </div>
      {selected && (
        <section className="detail-panel">
          <Head title={`${selected.title} · 阶段`} sub="按学习顺序组织路线" />
          <div className="inline-form compact">
            <input
              value={stage}
              onChange={(e) => setStage(e.target.value)}
              placeholder="阶段名称，例如 02 MDP"
            />
            <button
              onClick={async () => {
                if (!stage) return;
                await call("create_stage", {
                  planId: selected.id,
                  title: stage,
                });
                setStage("");
                setStages(await call("list_stages", { planId: selected.id }));
              }}
            >
              <Plus />
              添加阶段
            </button>
          </div>
          {stages.map((s) => (
            <div className="stage-row" key={s.id}>
              <div>
                <strong>{s.title}</strong>
                <span>{s.description || "尚未添加说明"}</span>
              </div>
              <b>{s.total ? Math.round((s.done / s.total) * 100) : 0}%</b>
            </div>
          ))}
        </section>
      )}
    </Page>
  );
}
function Projects() {
  const [list, setList] = useState<Project[]>([]),
    [title, setTitle] = useState(""),
    [selected, setSelected] = useState<Project | null>(null),
    [tasks, setTasks] = useState<Task[]>([]),
    [task, setTask] = useState(""),
    [important, setImportant] = useState(false),
    [projectStart,setProjectStart]=useState(today()),[projectEnd,setProjectEnd]=useState(""),
    [taskStart,setTaskStart]=useState(today()),[taskEnd,setTaskEnd]=useState(""),[taskDue,setTaskDue]=useState("");
  const load = () => call<Project[]>("list_projects").then(setList);
  useEffect(() => {
    void load();
  }, []);
  useEffect(() => {
    if (selected)
      call<Task[]>("project_tasks", { projectId: selected.id }).then(setTasks);
  }, [selected]);
  const refresh = () =>
    selected &&
    call<Task[]>("project_tasks", { projectId: selected.id }).then(setTasks);
  return (
    <Page
      over="项目工作台"
      title="项目"
      sub="参与一个项目，规划目标、时间和需要推进的重点任务。"
    >
      <div className="inline-form">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="新项目名称"
        />
        <input type="date" value={projectStart} onChange={e=>setProjectStart(e.target.value)}/>
        <input type="date" value={projectEnd} onChange={e=>setProjectEnd(e.target.value)}/>
        <button
          onClick={async () => {
            if (!title.trim()) return;
            await call("create_project", {
              input: { title, startDate: projectStart, endDate: projectEnd || null },
            });
            setTitle("");
            load();
          }}
        >
          <Plus />
          创建项目
        </button>
      </div>
      <div className="project-layout">
        <aside className="project-list">
          {list.map((p) => {
            const n = p.total ? Math.round((p.done / p.total) * 100) : 0;
            return (
              <div className="project-entry" key={p.id}>
              <button
                className={selected?.id === p.id ? "active" : ""}
                onClick={() => setSelected(p)}
              >
                <FolderKanban />
                <div>
                  <strong>{p.title}</strong>
                  <span>
                    {p.done}/{p.total} · {n}%
                  </span>
                </div>
              </button>
              <div className="row-actions"><button title="重命名" onClick={async()=>{const title=prompt("修改项目名称",p.title);if(title&&title!==p.title){await call("rename_project",{id:p.id,title});if(selected?.id===p.id)setSelected({...p,title});load()}}}>编辑</button><button className="danger" title="删除项目" onClick={async()=>{if(confirm(`删除项目“${p.title}”？项目中的任务会保留。`)){await call("delete_project",{id:p.id});if(selected?.id===p.id)setSelected(null);load()}}}><Trash2/></button></div>
              </div>
            );
          })}
        </aside>
        <section className="detail-panel project-detail">
          {selected ? (
            <>
              <Head
                title={selected.title}
                sub={
                  selected.description ||
                  `${selected.startDate ?? "未设置"} → ${selected.endDate ?? "进行中"}`
                }
              />
              <Progress
                n={
                  selected.total
                    ? Math.round((selected.done / selected.total) * 100)
                    : 0
                }
              />
              <div className="project-task-add">
                <input
                  value={task}
                  onChange={(e) => setTask(e.target.value)}
                  placeholder="添加项目任务…"
                />
                <input type="date" title="任务开始" value={taskStart} onChange={e=>setTaskStart(e.target.value)}/>
                <input type="date" title="任务结束" value={taskEnd} onChange={e=>setTaskEnd(e.target.value)}/>
                <input type="date" title="截止时间" value={taskDue} onChange={e=>setTaskDue(e.target.value)}/>
                <label>
                  <input
                    type="checkbox"
                    checked={important}
                    onChange={(e) => setImportant(e.target.checked)}
                  />
                  重点
                </label>
                <button
                  onClick={async () => {
                    if (!task.trim()) return;
                    await call("create_task", {
                      input: {
                        title: task,
                        projectId: selected.id,
                        isImportant: important,
                        priority: important ? "high" : "medium",
                        scheduleStart: taskStart,
                        scheduleEnd: taskEnd || null,
                        dueAt: taskDue || null,
                        recurrence: "none",
                        remindOnOpen: true,
                      },
                    });
                    setTask("");
                    setImportant(false);
                    refresh();
                  }}
                >
                  <Plus />
                  添加
                </button>
              </div>
              <div className="project-tasks">
                {tasks.map((t) => (
                  <article
                    key={t.id}
                    className={t.isImportant ? "important" : ""}
                  >
                    <button
                      onClick={async () => {
                        await call("set_task_status", {
                          id: t.id,
                          status: t.status === "done" ? "todo" : "done",
                        });
                        refresh();
                      }}
                    >
                      {t.status === "done" ? <CheckCircle2 /> : <Circle />}
                    </button>
                    <strong>{t.title}</strong>
                    {t.isImportant && <span>★ 重点</span>}
                  </article>
                ))}
              </div>
            </>
          ) : (
            <Empty text="选择一个项目开始规划。" />
          )}
        </section>
      </div>
    </Page>
  );
}

function Knowledge() {
  const [docs, setDocs] = useState<Doc[]>([]),
    [active, setActive] = useState<Doc | null>(null),
    [folder, setFolder] = useState<Doc | null>(null),
    [content, setContent] = useState(""),
    [mode, setMode] = useState<"edit" | "split" | "preview">("split"),
    [saved, setSaved] = useState(true);
  const load = () => call<Doc[]>("list_documents").then(setDocs);
  useEffect(() => {
    void load();
  }, []);
  const open = async (d: Doc) => {
      if (d.kind === "folder") {
        setFolder(d);
        setActive(null);
        return;
      }
      setActive(d);
      setContent(
        (await call<{ content: string }>("read_document", { id: d.id }))
          .content,
      );
      setSaved(true);
    },
    create = async (kind: string) => {
      const title = prompt(kind === "folder" ? "文件夹名称" : "笔记标题");
      if (!title) return;
      const id = await call<string>("create_document", {
        input: { title, kind, parentId: folder?.id ?? null },
      });
      const all = await call<Doc[]>("list_documents");
      setDocs(all);
      const d = all.find((x) => x.id === id);
      if (d && kind === "note") open(d);
    },
    save = useCallback(async () => {
      if (active) {
        await call("save_document", { id: active.id, content });
        setSaved(true);
      }
    }, [active, content]);
  useEffect(() => {
    const k = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "s") {
        e.preventDefault();
        save();
      }
    };
    addEventListener("keydown", k);
    return () => removeEventListener("keydown", k);
  }, [save]);
  useEffect(() => {
    if (saved || !active) return;
    const t = setTimeout(save, 900);
    return () => clearTimeout(t);
  }, [saved, active, content, save]);
  const pasteImage = async (
    event: React.ClipboardEvent<HTMLTextAreaElement>,
  ) => {
    const file = Array.from(event.clipboardData.files).find((x) =>
      x.type.startsWith("image/"),
    );
    if (!file) return;
    event.preventDefault();
    const result = await call<{ markdownPath: string }>("save_note_image", {
      fileName: file.name || "clipboard.png",
      bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
    });
    const area = event.currentTarget;
    const markdown = `![图片](${result.markdownPath})`;
    setContent(
      `${content.slice(0, area.selectionStart)}${markdown}${content.slice(area.selectionEnd)}`,
    );
    setSaved(false);
  };
  return (
    <div className="knowledge-layout">
      <aside className="doc-tree">
        <div className="doc-tree-head">
          <strong>知识库</strong>
          <div>
            <button onClick={() => create("folder")}>
              <FolderPlus />
            </button>
            <button onClick={() => create("note")}>
              <FilePlus2 />
            </button>
          </div>
        </div>
        {docs.map((d) => (
          <button
            key={d.id}
            className={`doc-item ${active?.id === d.id || folder?.id === d.id ? "active" : ""}`}
            style={{ paddingLeft: d.parentId ? 28 : 12 }}
            onClick={() => open(d)}
          >
            {d.kind === "folder" ? <FolderKanban /> : <BookOpen />}
            <span>{d.title}</span>
            {d.kind !== "folder" && (
              <button
                className="mini-trash"
                onClick={async (e) => {
                  e.stopPropagation();
                  await call("delete_document", { id: d.id });
                  setActive(null);
                  load();
                }}
              >
                <Trash2 />
              </button>
            )}
          </button>
        ))}
      </aside>
      <section className="editor-area">
        {active ? (
          <>
            <header className="editor-head">
              <div>
                <input value={active.title} readOnly />
                <span>
                  {saved ? "已保存" : "正在保存…"} · {active.relativePath}
                </span>
              </div>
              <div className="segmented">
                {(["edit", "split", "preview"] as const).map((x) => (
                  <button
                    key={x}
                    className={mode === x ? "active" : ""}
                    onClick={() => setMode(x)}
                  >
                    {x === "edit" ? "编辑" : x === "split" ? "分屏" : "预览"}
                  </button>
                ))}
                <button onClick={save}>
                  <Save />
                  保存
                </button>
              </div>
            </header>
            <div className={`markdown-workbench mode-${mode}`}>
              {mode !== "preview" && (
                <textarea
                  value={content}
                  onPaste={pasteImage}
                  onChange={(e) => {
                    setContent(e.target.value);
                    setSaved(false);
                  }}
                />
              )}
              {mode !== "edit" && (
                <article className="markdown-preview">
                  <ReactMarkdown
                    remarkPlugins={[remarkGfm, remarkMath]}
                    rehypePlugins={[rehypeKatex]}
                  >
                    {content}
                  </ReactMarkdown>
                </article>
              )}
            </div>
          </>
        ) : (
          <Empty text="新建或选择一篇笔记，正文将保存为真实 .md 文件。" />
        )}
      </section>
    </div>
  );
}
function UploadedKnowledge() {
  const [cats,setCats]=useState<KnowledgeCategory[]>([]),[selected,setSelected]=useState<KnowledgeCategory|null>(null),[files,setFiles]=useState<KnowledgeFile[]>([]);
  const load=()=>call<KnowledgeCategory[]>("list_knowledge_categories").then(setCats);
  useEffect(()=>{void load()},[]);
  useEffect(()=>{if(selected)call<KnowledgeFile[]>("list_knowledge_files",{categoryId:selected.id}).then(setFiles)},[selected]);
  const big=cats.filter(c=>!c.parentId), small=(id:string)=>cats.filter(c=>c.parentId===id);
  const rename=async(c:KnowledgeCategory)=>{const title=prompt("修改分类名称",c.title);if(title&&title!==c.title){await call("rename_knowledge_category",{id:c.id,title});if(selected?.id===c.id)setSelected({...c,title});load()}};
  const remove=async(c:KnowledgeCategory)=>{if(confirm(`删除分类“${c.title}”？分类将进入软删除状态，E 盘原始文档不会被物理删除。`)){await call("delete_knowledge_category",{id:c.id});if(selected?.id===c.id||selected?.parentId===c.id){setSelected(null);setFiles([])}load()}};
  return <Page over="E:\\朝暮数据\\knowledge" title="知识库" sub="创建两级分类，把从语雀等平台导出的文档上传到小知识库；原文件保存在 E 盘。">
    <div className="knowledge-toolbar"><div><strong>{big.length}</strong><span>大知识库</span></div><div><strong>{cats.length-big.length}</strong><span>小知识库</span></div><button onClick={async()=>{const title=prompt("大知识库类别名称");if(title){await call("create_knowledge_category",{title,parentId:null});load()}}}><FolderPlus/>新建大知识库</button></div>
    <div className="knowledge-layout"><aside className="knowledge-tree">{big.map(b=><section className="knowledge-group" key={b.id}><header><BookOpen/><strong>{b.title}</strong><div className="row-actions"><button onClick={()=>rename(b)}>编辑</button><button className="danger" onClick={()=>remove(b)}><Trash2/></button></div></header>{small(b.id).map(s=><div className={`knowledge-child ${selected?.id===s.id?"active":""}`} key={s.id}><button onClick={()=>setSelected(s)}><span className="tree-line">└</span><strong>{s.title}</strong><small>{selected?.id===s.id?files.length:""}</small></button><div className="row-actions"><button onClick={()=>rename(s)}>编辑</button><button className="danger" onClick={()=>remove(s)}><Trash2/></button></div></div>)}<button className="add-subcategory" onClick={async()=>{const title=prompt(`在“${b.title}”中新建小知识库类别`);if(title){await call("create_knowledge_category",{title,parentId:b.id});load()}}}><Plus/>添加小类别</button></section>)}{!big.length&&<Empty text="创建第一个大知识库，开始整理你的文档。"/>}</aside>
    <section className="detail-panel knowledge-content">{selected?<><div className="knowledge-content-head"><Head title={selected.title} sub="可一次选择多个文档，数量不限；单个文件最大 100MB。"/><label className="upload-button"><FilePlus2/>上传文档<input hidden type="file" multiple onChange={async e=>{for(const f of Array.from(e.target.files??[])){await call("upload_knowledge_file",{categoryId:selected.id,fileName:f.name,bytes:Array.from(new Uint8Array(await f.arrayBuffer()))})}setFiles(await call("list_knowledge_files",{categoryId:selected.id}));e.target.value=""}}/></label></div><div className="document-grid">{files.map(f=><article className="document-card" key={f.id}><div className="document-icon"><BookOpen/></div><div><strong>{f.fileName}</strong><span>{(f.fileSize/1024).toFixed(1)} KB</span><small>{f.relativePath}</small></div></article>)}</div>{!files.length&&<Empty text="这个分类还没有文档，点击右上角上传。"/>}</>:<Empty text="从左侧选择一个小知识库类别，再上传文档。"/>}</section></div>
  </Page>
}

function YuqueKnowledge() {
  void Knowledge;
  const [host, setHost] = useState("https://www.yuque.com"),
    [login, setLogin] = useState(""),
    [repo, setRepo] = useState(""),
    [token, setToken] = useState(""),
    [configured, setConfigured] = useState(false),
    [docs, setDocs] = useState<
      { id: number; title: string; slug: string; updatedAt: string }[]
    >([]),
    [error, setError] = useState(""),
    [title, setTitle] = useState("");
  const load = async () => {
    try {
      setError("");
      setDocs(await call("yuque_list_docs"));
    } catch (e) {
      setError(String(e));
    }
  };
  useEffect(() => {
    call<{
      host: string;
      login: string;
      repo: string;
      tokenConfigured: boolean;
    }>("get_yuque_config").then((x) => {
      setHost(x.host);
      setLogin(x.login);
      setRepo(x.repo);
      setConfigured(x.tokenConfigured);
      if (x.tokenConfigured && x.login && x.repo) load();
    });
  }, []);
  const base = host.replace(/\/api\/v2$/, "").replace(/\/$/, "");
  return (
    <Page
      over="Yuque Connected"
      title="语雀知识库"
      sub="笔记正文保存在语雀，朝暮只读取文档列表并提供任务联动入口。"
    >
      <section className="yuque-config">
        <Head
          title="语雀连接"
          sub="Token 只保存在本机 SQLite，不会发送给其他服务。"
        />
        <div className="config-grid">
          <label>
            站点地址
            <input value={host} onChange={(e) => setHost(e.target.value)} />
          </label>
          <label>
            账号 / 空间路径
            <input value={login} onChange={(e) => setLogin(e.target.value)} />
          </label>
          <label>
            知识库路径
            <input value={repo} onChange={(e) => setRepo(e.target.value)} />
          </label>
          <label>
            Personal / Team Token
            <input
              type="password"
              value={token}
              placeholder={
                configured ? "已保存；留空表示不修改" : "请输入语雀 Token"
              }
              onChange={(e) => setToken(e.target.value)}
            />
          </label>
        </div>
        <div className="yuque-actions">
          <button
            onClick={async () => {
              try {
                await call("save_yuque_config", { host, login, repo, token });
                setConfigured(true);
                setToken("");
                await load();
              } catch (e) {
                setError(String(e));
              }
            }}
          >
            <Save />
            保存并连接
          </button>
          <button onClick={load}>刷新文档</button>
          <a href={`${base}/${login}/${repo}`} target="_blank" rel="noreferrer">
            在语雀打开知识库
          </a>
        </div>
        {error && <p className="error-copy">{error}</p>}
      </section>
      <section className="yuque-docs">
        <div className="section-heading">
          <div>
            <h2>文档</h2>
            <p>{docs.length} 篇 · 内容由语雀托管</p>
          </div>
          <div className="new-yuque">
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="新文档标题"
            />
            <button
              onClick={async () => {
                if (!title.trim()) return;
                try {
                  const x = await call<{ url: string }>("yuque_create_doc", {
                    title,
                    body: `# ${title}\n\n`,
                  });
                  setTitle("");
                  await load();
                  window.open(x.url, "_blank");
                } catch (e) {
                  setError(String(e));
                }
              }}
            >
              <FilePlus2 />
              新建
            </button>
          </div>
        </div>
        <div className="yuque-list">
          {docs.map((d) => (
            <a
              key={d.id}
              href={`${base}/${login}/${repo}/${d.slug}`}
              target="_blank"
              rel="noreferrer"
            >
              <BookOpen />
              <div>
                <strong>{d.title}</strong>
                <span>{d.updatedAt ?? "语雀文档"}</span>
              </div>
              <ChevronRight />
            </a>
          ))}
        </div>
        {configured && !docs.length && !error && (
          <Empty text="该知识库还没有文档。" />
        )}
      </section>
    </Page>
  );
}

function Checkin() {
  const [mood, setMood] = useState(3),
    [summary, setSummary] = useState(""),
    [saved, setSaved] = useState(false),
    [s, setS] = useState<Stats | null>(null);
  useEffect(() => {
    call<Stats>("analytics").then(setS);
    call<{ mood: number; summary: string } | null>("get_checkin", {
      date: today(),
    }).then((x) => {
      if (x) {
        setMood(x.mood);
        setSummary(x.summary);
        setSaved(true);
      }
    });
  }, []);
  return (
    <Page
      over={`🔥 ${today()}`}
      title="今日学习总结"
      sub="一天一次，回看完成、专注与收获。"
    >
      <div className="checkin-grid">
        <section className="quiet-card">
          <Head title="今日状态" sub="选择最接近此刻的感受" />
          <div className="moods">
            {["😫", "😕", "😐", "🙂", "😄"].map((x, i) => (
              <button
                key={x}
                className={mood === i + 1 ? "active" : ""}
                onClick={() => setMood(i + 1)}
              >
                {x}
              </button>
            ))}
          </div>
          <label className="field">
            <span>今日总结（支持 Markdown）</span>
            <textarea
              value={summary}
              onChange={(e) => {
                setSummary(e.target.value);
                setSaved(false);
              }}
            />
          </label>
          <button
            className="primary-button"
            onClick={async () => {
              await call("save_checkin", { date: today(), mood, summary });
              setSaved(true);
            }}
          >
            <Flame />
            {saved ? "更新今日打卡" : "完成今日打卡"}
          </button>
        </section>
        <section className="today-summary">
          <Metric
            icon={<Clock3 />}
            label="今日学习"
            value={fmt(s?.todayMinutes ?? 0)}
          />
          <Metric
            icon={<Flame />}
            label="今年学习天数"
            value={`${s?.studyDays ?? 0} 天`}
          />
        </section>
      </div>
      <Daily />
    </Page>
  );
}
function Daily({ compact = false }: { compact?: boolean }) {
  const [content, setContent] = useState(""),
    [saved, setSaved] = useState(true);
  useEffect(() => {
    call<{ content: string }>("daily_note", { date: today() }).then((x) =>
      setContent(x.content),
    );
  }, []);
  return (
    <article className={`quiet-card daily-card ${compact ? "compact" : ""}`}>
      <Head
        title="今日 Daily Note"
        sub={`daily/${today().slice(0, 4)}/${today().slice(5, 7)}/${today()}.md`}
      />
      {!compact && (
        <textarea
          value={content}
          onChange={(e) => {
            setContent(e.target.value);
            setSaved(false);
          }}
        />
      )}
      <button
        className="text-button"
        onClick={async () => {
          if (compact) {
            location.hash = "#/check-in";
            return;
          }
          await call("save_daily_note", { date: today(), content });
          setSaved(true);
        }}
      >
        {compact
          ? "打开今天的 Daily Note"
          : saved
            ? "已保存"
            : "保存 Daily Note"}
      </button>
    </article>
  );
}
function Calendar() {
  const [cur, setCur] = useState(new Date()),
    [data, setData] = useState<Cal[]>([]),
    month = `${cur.getFullYear()}-${String(cur.getMonth() + 1).padStart(2, "0")}`;
  useEffect(() => {
    call<Cal[]>("calendar_month", { month }).then(setData);
  }, [month]);
  const blanks = new Date(cur.getFullYear(), cur.getMonth(), 1).getDay(),
    count = new Date(cur.getFullYear(), cur.getMonth() + 1, 0).getDate(),
    cells = [
      ...Array(blanks).fill(null),
      ...Array.from({ length: count }, (_, i) => i + 1),
    ];
  return (
    <Page over="月视图" title="日历" sub="查看每日任务、学习时间与打卡。">
      <div className="calendar-toolbar">
        <button
          onClick={() =>
            setCur(new Date(cur.getFullYear(), cur.getMonth() - 1))
          }
        >
          <ChevronLeft />
        </button>
        <strong>
          {cur.getFullYear()}年 {cur.getMonth() + 1}月
        </strong>
        <button
          onClick={() =>
            setCur(new Date(cur.getFullYear(), cur.getMonth() + 1))
          }
        >
          <ChevronRight />
        </button>
      </div>
      <div className="calendar-grid">
        {["日", "一", "二", "三", "四", "五", "六"].map((x) => (
          <b className="weekday" key={x}>
            {x}
          </b>
        ))}
        {cells.map((n, i) => {
          if (!n) return <span key={i} />;
          const date = `${month}-${String(n).padStart(2, "0")}`,
            x = data.find((d) => d.date === date);
          return (
            <article
              key={date}
              className={`day-cell ${date === today() ? "today" : ""}`}
            >
              <strong>{n}</strong>
              {x && (
                <>
                  <span>
                    ✓ {x.done}/{x.total}
                  </span>
                  <span>{fmt(x.minutes)}</span>
                  {x.checkedIn && <i>🔥</i>}
                </>
              )}
            </article>
          );
        })}
      </div>
    </Page>
  );
}
function Analytics() {
  const [s, setS] = useState<Stats | null>(null);
  useEffect(() => {
    call<Stats>("analytics").then(setS);
  }, []);
  return (
    <Page
      over="专注与积累"
      title="数据统计"
      sub="所有趋势均由本地学习记录生成。"
    >
      <div className="stat-grid">
        <Stat l="今日学习" v={fmt(s?.todayMinutes ?? 0)} />
        <Stat l="本周学习" v={fmt(s?.weekMinutes ?? 0)} />
        <Stat l="本月学习" v={fmt(s?.monthMinutes ?? 0)} />
        <Stat l="今年学习" v={`${s?.studyDays ?? 0} 天`} />
      </div>
      <section className="chart-card">
        <Head title="本周学习趋势" sub="最近 7 天学习分钟数" />
        <ResponsiveContainer width="100%" height={280}>
          <AreaChart data={s?.trend ?? []}>
            <CartesianGrid strokeDasharray="3 3" vertical={false} />
            <XAxis dataKey="date" tickFormatter={(x) => x.slice(5)} />
            <YAxis />
            <Tooltip />
            <Area dataKey="minutes" stroke="#587a6c" fill="#dce9e3" />
          </AreaChart>
        </ResponsiveContainer>
      </section>
    </Page>
  );
}
function Trash() {
  const [items, setItems] = useState<
      { type: string; id: string; title: string; deletedAt: string }[]
    >([]),
    load = () => call<typeof items>("trash_items").then(setItems);
  useEffect(() => {
    void load();
  }, []);
  return (
    <Page
      over="软删除保护"
      title="回收站"
      sub="误删内容可以恢复，Markdown 文件不会随软删除消失。"
    >
      <div className="managed-list">
        {items.map((x) => (
          <div className="trash-row" key={`${x.type}-${x.id}`}>
            <Trash2 />
            <div>
              <strong>{x.title}</strong>
              <span>
                {x.type} · {x.deletedAt}
              </span>
            </div>
            <button
              onClick={async () => {
                await call("restore_item", { itemType: x.type, id: x.id });
                load();
              }}
            >
              <ArchiveRestore />
              恢复
            </button>
          </div>
        ))}
      </div>
      {!items.length && <Empty text="回收站是空的。" />}
    </Page>
  );
}
function SettingsPage({ boot }: { boot: Boot | null }) {
  const [msg, setMsg] = useState(""),
    [mode, setMode] = useState("daily"),
    [retain, setRetain] = useState(7),
    [notesRoot, setNotesRoot] = useState("");
  useEffect(() => {
    call<{ backupMode: string; backupRetain: number; notesRoot: string }>(
      "get_preferences",
    ).then((x) => {
      setMode(x.backupMode);
      setRetain(x.backupRetain);
      setNotesRoot(x.notesRoot);
    });
  }, []);
  return (
    <Page
      over="Local First"
      title="设置"
      sub="数据库、Markdown、图片与备份全部保存在本机。"
    >
      <section className="settings-list">
        <div>
          <strong>本地数据目录</strong>
          <code>{boot?.dataRoot}</code>
        </div>
        <div>
          <strong>SQLite 数据库</strong>
          <code>{boot?.databasePath}</code>
        </div>
        <div>
          <strong>知识库 / 笔记根目录</strong>
          <p>修改后会把现有 Markdown 笔记安全复制到新目录。</p>
          <div className="root-setting">
            <input
              value={notesRoot}
              onChange={(e) => setNotesRoot(e.target.value)}
            />
            <button
              onClick={async () => {
                try {
                  await call("set_notes_root", { path: notesRoot });
                  setMsg("笔记目录已更新");
                } catch (e) {
                  setMsg(String(e));
                }
              }}
            >
              应用目录
            </button>
          </div>
        </div>
        <div>
          <strong>完整备份</strong>
          <p>包含数据库、notes、daily 与 assets。</p>
          <div className="preference-row">
            <label>自动备份</label>
            <select value={mode} onChange={(e) => setMode(e.target.value)}>
              <option value="daily">每天</option>
              <option value="weekly">每周</option>
              <option value="off">关闭</option>
            </select>
            <label>保留份数</label>
            <input
              type="number"
              min="1"
              max="50"
              value={retain}
              onChange={(e) => setRetain(Number(e.target.value))}
            />
            <button
              onClick={async () => {
                await call("save_preferences", {
                  backupMode: mode,
                  backupRetain: retain,
                });
                setMsg("自动备份设置已保存");
              }}
            >
              保存设置
            </button>
          </div>
          <button
            className="primary-button"
            onClick={async () => {
              setMsg("正在备份…");
              try {
                setMsg(`备份完成：${await call<string>("create_backup")}`);
              } catch (e) {
                setMsg(String(e));
              }
            }}
          >
            <Save />
            立即备份
          </button>
          <small>{msg}</small>
        </div>
      </section>
    </Page>
  );
}
function SearchBox({
  close,
  go,
}: {
  close: () => void;
  go: (x: string) => void;
}) {
  const [q, setQ] = useState(""),
    [hits, setHits] = useState<Hit[]>([]);
  useEffect(() => {
    const t = setTimeout(
      () =>
        q.trim()
          ? call<Hit[]>("global_search", { query: q }).then(setHits)
          : setHits([]),
      180,
    );
    return () => clearTimeout(t);
  }, [q]);
  return (
    <div className="modal-backdrop" onMouseDown={close}>
      <section
        className="search-modal"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="modal-search">
          <Search />
          <input
            autoFocus
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="搜索任务、计划、笔记…"
          />
          <button onClick={close}>
            <X />
          </button>
        </div>
        <div className="search-results">
          {hits.map((x) => (
            <button
              key={`${x.type}-${x.id}`}
              onClick={() => {
                go(
                  x.type === "task"
                    ? "/tasks"
                    : x.type === "plan"
                      ? "/plans"
                      : "/knowledge",
                );
                close();
              }}
            >
              <span>
                {x.type === "task"
                  ? "任务"
                  : x.type === "plan"
                    ? "计划"
                    : "笔记"}
              </span>
              <div>
                <strong>{x.title}</strong>
                <small>{x.subtitle}</small>
              </div>
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}
function Study({
  task,
  close,
  done,
}: {
  task: Task;
  close: () => void;
  done: () => void;
}) {
  const [min, setMin] = useState(String(task.estimatedMinutes ?? 30)),
    [note, setNote] = useState("");
  return (
    <div className="modal-backdrop">
      <section className="dialog">
        <button className="dialog-close" onClick={close}>
          <X />
        </button>
        <p className="eyebrow">完成并记录学习</p>
        <h2>{task.title}</h2>
        <label className="field">
          <span>学习时间（分钟）</span>
          <input value={min} onChange={(e) => setMin(e.target.value)} />
        </label>
        <label className="field">
          <span>学习内容与备注</span>
          <textarea value={note} onChange={(e) => setNote(e.target.value)} />
        </label>
        <div className="dialog-actions">
          <button
            onClick={async () => {
              await call("set_task_status", { id: task.id, status: "done" });
              done();
            }}
          >
            仅完成
          </button>
          <button
            className="primary-button"
            onClick={async () => {
              await call("record_study", {
                taskId: task.id,
                minutes: Number(min),
                content: task.title,
                note,
              });
              done();
            }}
          >
            <Flame />
            完成并记录
          </button>
        </div>
      </section>
    </div>
  );
}
function Page(p: {
  over: string;
  title: string;
  sub: string;
  children: React.ReactNode;
}) {
  return (
    <div className="page">
      <div className="page-intro">
        <p className="eyebrow">{p.over}</p>
        <h1>{p.title}</h1>
        <p>{p.sub}</p>
      </div>
      {p.children}
    </div>
  );
}
function Head({ title, sub }: { title: string; sub: string }) {
  return (
    <div className="section-heading">
      <div>
        <h2>{title}</h2>
        <p>{sub}</p>
      </div>
    </div>
  );
}
function Progress({ n }: { n: number }) {
  return (
    <div className="progress-track">
      <span style={{ width: `${n}%` }} />
    </div>
  );
}
function Metric({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="metric">
      {icon}
      <div>
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
    </div>
  );
}
function Stat({ l, v }: { l: string; v: string }) {
  return (
    <article className="stat-card">
      <span>{l}</span>
      <strong>{v}</strong>
    </article>
  );
}
function Empty({ text }: { text: string }) {
  return (
    <div className="empty-state">
      <CheckCircle2 />
      <strong>{text}</strong>
    </div>
  );
}
function Composer(p: {
  title: string;
  setTitle: (x: string) => void;
  minutes: string;
  setMinutes: (x: string) => void;
  add: () => void;
}) {
  return (
    <div className="task-composer">
      <input
        value={p.title}
        onChange={(e) => p.setTitle(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && p.add()}
        placeholder="添加一个任务…"
      />
      <input
        className="minutes-input"
        value={p.minutes}
        onChange={(e) => p.setMinutes(e.target.value)}
      />
      <span>min</span>
      <button onClick={p.add}>
        <Plus />
        添加
      </button>
    </div>
  );
}
export default function App() {
  void Plans;
  void YuqueKnowledge;
  return (
    <HashRouter>
      <Shell />
    </HashRouter>
  );
}
