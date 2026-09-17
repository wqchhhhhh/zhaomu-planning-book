use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{Manager, Runtime};

const INITIAL_MIGRATION: &str = include_str!("../migrations/001_initial.sql");
const PROJECTS_MIGRATION: &str = include_str!("../migrations/002_projects.sql");
const SCHEDULES_MIGRATION: &str = include_str!("../migrations/003_task_schedules_knowledge.sql");

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppBootstrap {
    data_root: String,
    database_path: String,
    schema_version: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Task {
    id: String,
    title: String,
    description: String,
    status: String,
    priority: String,
    planned_date: Option<String>,
    due_at: Option<String>,
    estimated_minutes: Option<i64>,
    actual_minutes: i64,
    plan_id: Option<String>,
    stage_id: Option<String>,
    document_id: Option<String>,
    completed_at: Option<String>,
    is_important: bool,
    project_id: Option<String>,
    schedule_start: Option<String>,
    schedule_end: Option<String>,
    recurrence: String,
    remind_on_open: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTask {
    title: String,
    description: Option<String>,
    planned_date: Option<String>,
    priority: Option<String>,
    estimated_minutes: Option<i64>,
    plan_id: Option<String>,
    stage_id: Option<String>,
    due_at: Option<String>,
    is_important: Option<bool>,
    project_id: Option<String>,
    schedule_start: Option<String>,
    schedule_end: Option<String>,
    recurrence: Option<String>,
    remind_on_open: Option<bool>,
}

fn initialize_storage<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<AppBootstrap, String> {
    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    let data_root = app_data.join("PlannerData");

    for relative in [
        "database",
        "notes",
        "daily",
        "assets/images",
        "assets/attachments",
        "backups",
        "config",
    ] {
        fs::create_dir_all(data_root.join(relative)).map_err(|error| error.to_string())?;
    }

    let database_path = data_root.join("database").join("planner.db");
    let connection = Connection::open(&database_path).map_err(|error| error.to_string())?;
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON;
       PRAGMA journal_mode=WAL;
       PRAGMA synchronous=FULL;
       PRAGMA busy_timeout=5000;",
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute_batch(INITIAL_MIGRATION)
        .map_err(|error| error.to_string())?;
    let version: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if version < 2 {
        connection
            .execute_batch(PROJECTS_MIGRATION)
            .map_err(|e| e.to_string())?;
    }
    let version: i64 = connection.query_row("SELECT COALESCE(MAX(version),0) FROM schema_migrations", [], |r| r.get(0)).map_err(|e|e.to_string())?;
    if version < 3 {
        connection.execute_batch(SCHEDULES_MIGRATION).map_err(|e|e.to_string())?;
    }
    fs::create_dir_all(knowledge_root()).map_err(|e| format!("无法创建 E 盘知识库目录：{e}"))?;

    let schema_version = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    Ok(AppBootstrap {
        data_root: path_to_string(data_root),
        database_path: path_to_string(database_path),
        schema_version,
    })
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn open_database<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<Connection, String> {
    let database_path = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("PlannerData/database/planner.db");
    let connection = Connection::open(database_path).map_err(|error| error.to_string())?;
    connection
        .execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

fn data_root<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("PlannerData"))
}

fn notes_root<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let fallback = data_root(app)?.join("notes");
    let db = open_database(app)?;
    Ok(db
        .query_row(
            "SELECT value_json FROM settings WHERE key='notes_root'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .filter(|x| !x.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(fallback))
}

fn knowledge_root() -> PathBuf { PathBuf::from(r"E:\朝暮数据\knowledge") }

fn new_id(connection: &Connection) -> Result<String, String> {
    connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))
        .map_err(|e| e.to_string())
}

fn safe_name(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() {
        "未命名".into()
    } else {
        cleaned.chars().take(80).collect()
    }
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    let old = path.with_extension("bak");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    if path.exists() {
        let _ = fs::remove_file(&old);
        fs::rename(path, &old).map_err(|e| e.to_string())?;
    }
    if let Err(error) = fs::rename(&tmp, path) {
        if old.exists() {
            let _ = fs::rename(&old, path);
        }
        return Err(error.to_string());
    }
    let _ = fs::remove_file(old);
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(target).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let destination = target.join(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn read_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        priority: row.get(4)?,
        planned_date: row.get(5)?,
        due_at: row.get(6)?,
        estimated_minutes: row.get(7)?,
        actual_minutes: row.get(8)?,
        plan_id: row.get(9)?,
        stage_id: row.get(10)?,
        document_id: row.get(11)?,
        completed_at: row.get(12)?,
        is_important: row.get(13)?,
        project_id: row.get(14)?,
        schedule_start: row.get(15)?, schedule_end: row.get(16)?,
        recurrence: row.get(17)?, remind_on_open: row.get(18)?,
    })
}

#[tauri::command]
fn bootstrap(app: tauri::AppHandle) -> Result<AppBootstrap, String> {
    initialize_storage(&app)
}

#[tauri::command]
fn list_tasks(app: tauri::AppHandle, planned_date: Option<String>) -> Result<Vec<Task>, String> {
    let connection = open_database(&app)?;
    let sql = "SELECT id,title,description,status,priority,planned_date,due_at,estimated_minutes,
                    actual_minutes,plan_id,stage_id,document_id,completed_at,is_important,project_id,schedule_start,schedule_end,recurrence,remind_on_open
             FROM tasks
             WHERE deleted_at IS NULL AND (?1 IS NULL OR planned_date=?1 OR EXISTS(SELECT 1 FROM task_daily_items d WHERE d.task_id=tasks.id AND d.item_date=?1) OR (schedule_start<=?1 AND COALESCE(schedule_end,?1)>=?1 AND (recurrence='daily' OR (recurrence='weekdays' AND CAST(strftime('%w',?1) AS INTEGER) BETWEEN 1 AND 5) OR (recurrence='weekly' AND strftime('%w',schedule_start)=strftime('%w',?1)))))
             ORDER BY is_important DESC, CASE status WHEN 'doing' THEN 0 WHEN 'todo' THEN 1 ELSE 2 END, sort_order, created_at DESC";
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![planned_date], read_task)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_task(app: tauri::AppHandle, input: CreateTask) -> Result<String, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("任务标题不能为空".into());
    }
    let priority = input.priority.unwrap_or_else(|| "medium".into());
    if !["low", "medium", "high", "urgent"].contains(&priority.as_str()) {
        return Err("无效的优先级".into());
    }
    let connection = open_database(&app)?;
    let id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    connection.execute(
    "INSERT INTO tasks(id,title,description,status,priority,planned_date,estimated_minutes,plan_id,stage_id,due_at,is_important,project_id,schedule_start,schedule_end,recurrence,remind_on_open,created_at,updated_at)
     VALUES(?1,?2,?3,'todo',?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
    params![id, title, input.description.unwrap_or_default(), priority, input.planned_date, input.estimated_minutes,input.plan_id,input.stage_id,input.due_at,input.is_important.unwrap_or(false),input.project_id,input.schedule_start,input.schedule_end,input.recurrence.unwrap_or_else(||"none".into()),input.remind_on_open.unwrap_or(true)],
  ).map_err(|error| error.to_string())?;
    Ok(id)
}

#[tauri::command]
fn set_task_status(app: tauri::AppHandle, id: String, status: String) -> Result<(), String> {
    if !["todo", "doing", "done"].contains(&status.as_str()) {
        return Err("无效的任务状态".into());
    }
    let connection = open_database(&app)?;
    let changed = connection.execute(
    "UPDATE tasks SET status=?2, completed_at=CASE WHEN ?2='done' THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') ELSE NULL END,
     updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1 AND deleted_at IS NULL",
    params![id, status],
  ).map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("任务不存在".into());
    }
    Ok(())
}

#[tauri::command]
fn delete_task(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let connection = open_database(&app)?;
    connection.execute(
    "UPDATE tasks SET deleted_at=strftime('%Y-%m-%dT%H:%M:%fZ','now'), updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
    params![id],
  ).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn update_task(app: tauri::AppHandle, id: String, title: String) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("任务标题不能为空".into());
    }
    let db = open_database(&app)?;
    db.execute(
        "UPDATE tasks SET title=?2,updated_at=datetime('now') WHERE id=?1 AND deleted_at IS NULL",
        params![id, title.trim()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn reorder_tasks(app: tauri::AppHandle, ids: Vec<String>) -> Result<(), String> {
    let mut db = open_database(&app)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE tasks SET sort_order=?2,updated_at=datetime('now') WHERE id=?1",
            params![id, index as i64],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_task_important(app: tauri::AppHandle, id: String, important: bool) -> Result<(), String> {
    open_database(&app)?
        .execute(
            "UPDATE tasks SET is_important=?2,updated_at=datetime('now') WHERE id=?1",
            params![id, important],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)] #[serde(rename_all="camelCase")]
struct DailyItem { id:String, task_id:String, task_title:String, item_date:String, content:String, is_done:bool }

#[tauri::command]
fn list_task_daily_items(app:tauri::AppHandle, task_id:String)->Result<Vec<DailyItem>,String>{
    let db=open_database(&app)?;
    let mut s=db.prepare("SELECT d.id,d.task_id,t.title,d.item_date,d.content,d.is_done FROM task_daily_items d JOIN tasks t ON t.id=d.task_id WHERE d.task_id=?1 ORDER BY d.item_date,d.created_at").map_err(|e|e.to_string())?;
    let rows=s.query_map([task_id],|r|Ok(DailyItem{id:r.get(0)?,task_id:r.get(1)?,task_title:r.get(2)?,item_date:r.get(3)?,content:r.get(4)?,is_done:r.get(5)?})).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[tauri::command]
fn today_daily_items(app:tauri::AppHandle, date:String)->Result<Vec<DailyItem>,String>{
    let db=open_database(&app)?;
    let mut s=db.prepare("SELECT d.id,d.task_id,t.title,d.item_date,d.content,d.is_done FROM task_daily_items d JOIN tasks t ON t.id=d.task_id WHERE d.item_date=?1 AND t.deleted_at IS NULL ORDER BY t.is_important DESC,d.created_at").map_err(|e|e.to_string())?;
    let rows=s.query_map([date],|r|Ok(DailyItem{id:r.get(0)?,task_id:r.get(1)?,task_title:r.get(2)?,item_date:r.get(3)?,content:r.get(4)?,is_done:r.get(5)?})).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
}

#[tauri::command]
fn add_task_daily_item(app:tauri::AppHandle, task_id:String, item_date:String, content:String)->Result<String,String>{
    if content.trim().is_empty(){return Err("每日安排不能为空".into())} let db=open_database(&app)?; let id=new_id(&db)?;
    db.execute("INSERT INTO task_daily_items(id,task_id,item_date,content) VALUES(?1,?2,?3,?4)",params![id,task_id,item_date,content.trim()]).map_err(|e|e.to_string())?; Ok(id)
}

#[tauri::command]
fn set_daily_item_done(app:tauri::AppHandle,id:String,done:bool)->Result<(),String>{open_database(&app)?.execute("UPDATE task_daily_items SET is_done=?2,updated_at=datetime('now') WHERE id=?1",params![id,done]).map_err(|e|e.to_string())?;Ok(())}

#[derive(Serialize)] #[serde(rename_all="camelCase")]
struct KnowledgeCategory{id:String,parent_id:Option<String>,title:String}
#[derive(Serialize)] #[serde(rename_all="camelCase")]
struct KnowledgeFile{id:String,category_id:String,title:String,file_name:String,relative_path:String,file_size:i64}

#[tauri::command]
fn list_knowledge_categories(app:tauri::AppHandle)->Result<Vec<KnowledgeCategory>,String>{let db=open_database(&app)?;let mut s=db.prepare("SELECT id,parent_id,title FROM knowledge_categories WHERE deleted_at IS NULL ORDER BY parent_id IS NOT NULL,sort_order,created_at").map_err(|e|e.to_string())?;let rows=s.query_map([],|r|Ok(KnowledgeCategory{id:r.get(0)?,parent_id:r.get(1)?,title:r.get(2)?})).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())}

#[tauri::command]
fn create_knowledge_category(app:tauri::AppHandle,title:String,parent_id:Option<String>)->Result<String,String>{if title.trim().is_empty(){return Err("分类名称不能为空".into())}let db=open_database(&app)?;if let Some(ref p)=parent_id{let parent:Option<String>=db.query_row("SELECT parent_id FROM knowledge_categories WHERE id=?1 AND deleted_at IS NULL",[p],|r|r.get(0)).map_err(|_|"上级分类不存在".to_string())?;if parent.is_some(){return Err("仅支持大类和小类两级".into())}}let id=new_id(&db)?;db.execute("INSERT INTO knowledge_categories(id,parent_id,title) VALUES(?1,?2,?3)",params![id,parent_id,title.trim()]).map_err(|e|e.to_string())?;Ok(id)}

#[tauri::command]
fn rename_knowledge_category(app:tauri::AppHandle,id:String,title:String)->Result<(),String>{if title.trim().is_empty(){return Err("分类名称不能为空".into())}let db=open_database(&app)?;let changed=db.execute("UPDATE knowledge_categories SET title=?2,updated_at=datetime('now') WHERE id=?1 AND deleted_at IS NULL",params![id,title.trim()]).map_err(|e|e.to_string())?;if changed==0{return Err("分类不存在".into())}Ok(())}

#[tauri::command]
fn delete_knowledge_category(app:tauri::AppHandle,id:String)->Result<(),String>{let mut db=open_database(&app)?;let tx=db.transaction().map_err(|e|e.to_string())?;tx.execute("UPDATE knowledge_files SET deleted_at=datetime('now'),updated_at=datetime('now') WHERE category_id=?1 OR category_id IN (SELECT id FROM knowledge_categories WHERE parent_id=?1)",[&id]).map_err(|e|e.to_string())?;tx.execute("UPDATE knowledge_categories SET deleted_at=datetime('now'),updated_at=datetime('now') WHERE id=?1 OR parent_id=?1",[&id]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;Ok(())}

#[tauri::command]
fn list_knowledge_files(app:tauri::AppHandle,category_id:String)->Result<Vec<KnowledgeFile>,String>{let db=open_database(&app)?;let mut s=db.prepare("SELECT id,category_id,title,file_name,relative_path,file_size FROM knowledge_files WHERE category_id=?1 AND deleted_at IS NULL ORDER BY created_at DESC").map_err(|e|e.to_string())?;let rows=s.query_map([category_id],|r|Ok(KnowledgeFile{id:r.get(0)?,category_id:r.get(1)?,title:r.get(2)?,file_name:r.get(3)?,relative_path:r.get(4)?,file_size:r.get(5)?})).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())}

#[tauri::command]
fn upload_knowledge_file(app:tauri::AppHandle,category_id:String,file_name:String,bytes:Vec<u8>)->Result<String,String>{if bytes.len()>100*1024*1024{return Err("单个文档不能超过 100MB".into())}let db=open_database(&app)?;let (small,parent):(String,Option<String>)=db.query_row("SELECT title,parent_id FROM knowledge_categories WHERE id=?1 AND deleted_at IS NULL",[&category_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"小知识库分类不存在".to_string())?;let parent=parent.ok_or("文档只能上传到小知识库分类")?;let big:String=db.query_row("SELECT title FROM knowledge_categories WHERE id=?1",[parent],|r|r.get(0)).map_err(|e|e.to_string())?;let clean=safe_name(&file_name);let id=new_id(&db)?;let relative=PathBuf::from(safe_name(&big)).join(safe_name(&small)).join(format!("{}-{}",&id[..8],clean));atomic_write(&knowledge_root().join(&relative),&bytes)?;db.execute("INSERT INTO knowledge_files(id,category_id,title,file_name,relative_path,file_size) VALUES(?1,?2,?3,?4,?5,?6)",params![id,category_id,clean,file_name,path_to_string(relative),bytes.len() as i64]).map_err(|e|e.to_string())?;Ok(id)}

#[tauri::command]
fn open_reminders(app:tauri::AppHandle,date:String)->Result<Vec<String>,String>{let db=open_database(&app)?;let mut s=db.prepare("SELECT title FROM tasks WHERE deleted_at IS NULL AND status!='done' AND remind_on_open=1 AND (schedule_start=?1 OR due_at<=?1) ORDER BY is_important DESC,due_at").map_err(|e|e.to_string())?;let rows=s.query_map([date],|r|r.get(0)).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInput {
    title: String,
    description: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
}

#[tauri::command]
fn list_projects(app: tauri::AppHandle) -> Result<Vec<Value>, String> {
    let db = open_database(&app)?;
    let mut s=db.prepare("SELECT p.id,p.title,p.description,p.status,p.start_date,p.end_date,COUNT(t.id),COALESCE(SUM(CASE WHEN t.status='done' THEN 1 ELSE 0 END),0) FROM projects p LEFT JOIN tasks t ON t.project_id=p.id AND t.deleted_at IS NULL WHERE p.deleted_at IS NULL GROUP BY p.id ORDER BY p.sort_order,p.created_at DESC").map_err(|e|e.to_string())?;
    let rows=s.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"title":r.get::<_,String>(1)?,"description":r.get::<_,String>(2)?,"status":r.get::<_,String>(3)?,"startDate":r.get::<_,Option<String>>(4)?,"endDate":r.get::<_,Option<String>>(5)?,"total":r.get::<_,i64>(6)?,"done":r.get::<_,i64>(7)?}))).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_project(app: tauri::AppHandle, input: ProjectInput) -> Result<String, String> {
    let db = open_database(&app)?;
    let id = new_id(&db)?;
    db.execute("INSERT INTO projects(id,title,description,status,start_date,end_date,created_at,updated_at) VALUES(?1,?2,?3,'active',?4,?5,datetime('now'),datetime('now'))",params![id,input.title.trim(),input.description.unwrap_or_default(),input.start_date,input.end_date]).map_err(|e|e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn rename_project(app:tauri::AppHandle,id:String,title:String)->Result<(),String>{if title.trim().is_empty(){return Err("项目名称不能为空".into())}let changed=open_database(&app)?.execute("UPDATE projects SET title=?2,updated_at=datetime('now') WHERE id=?1 AND deleted_at IS NULL",params![id,title.trim()]).map_err(|e|e.to_string())?;if changed==0{return Err("项目不存在".into())}Ok(())}

#[tauri::command]
fn delete_project(app:tauri::AppHandle,id:String)->Result<(),String>{let mut db=open_database(&app)?;let tx=db.transaction().map_err(|e|e.to_string())?;tx.execute("UPDATE tasks SET project_id=NULL,updated_at=datetime('now') WHERE project_id=?1",[&id]).map_err(|e|e.to_string())?;tx.execute("UPDATE projects SET deleted_at=datetime('now'),updated_at=datetime('now') WHERE id=?1",[&id]).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;Ok(())}

#[tauri::command]
fn task_daily_progress(app:tauri::AppHandle)->Result<Vec<Value>,String>{let db=open_database(&app)?;let mut s=db.prepare("SELECT t.id,COUNT(d.id),COALESCE(SUM(d.is_done),0) FROM tasks t LEFT JOIN task_daily_items d ON d.task_id=t.id WHERE t.deleted_at IS NULL GROUP BY t.id").map_err(|e|e.to_string())?;let rows=s.query_map([],|r|Ok(json!({"taskId":r.get::<_,String>(0)?,"total":r.get::<_,i64>(1)?,"done":r.get::<_,i64>(2)?}))).map_err(|e|e.to_string())?;rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())}

#[tauri::command]
fn project_tasks(app: tauri::AppHandle, project_id: String) -> Result<Vec<Task>, String> {
    let db = open_database(&app)?;
    let mut s=db.prepare("SELECT id,title,description,status,priority,planned_date,due_at,estimated_minutes,actual_minutes,plan_id,stage_id,document_id,completed_at,is_important,project_id,schedule_start,schedule_end,recurrence,remind_on_open FROM tasks WHERE project_id=?1 AND deleted_at IS NULL ORDER BY is_important DESC,sort_order,created_at DESC").map_err(|e|e.to_string())?;
    let rows = s
        .query_map([project_id], read_task)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn record_study(
    app: tauri::AppHandle,
    task_id: Option<String>,
    minutes: i64,
    content: String,
    note: String,
) -> Result<String, String> {
    if minutes <= 0 {
        return Err("学习时间必须大于 0".into());
    }
    let mut db = open_database(&app)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let id = new_id(&tx)?;
    tx.execute("INSERT INTO study_sessions(id,task_id,started_at,ended_at,duration_minutes,content,note,created_at,updated_at) VALUES(?1,?2,datetime('now'),datetime('now'),?3,?4,?5,datetime('now'),datetime('now'))", params![id,task_id,minutes,content,note]).map_err(|e| e.to_string())?;
    if let Some(task) = task_id {
        tx.execute("UPDATE tasks SET actual_minutes=actual_minutes+?2,status='done',completed_at=datetime('now'),updated_at=datetime('now') WHERE id=?1",params![task,minutes]).map_err(|e|e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanInput {
    title: String,
    description: Option<String>,
    kind: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
}

#[tauri::command]
fn list_plans(app: tauri::AppHandle) -> Result<Vec<Value>, String> {
    let db = open_database(&app)?;
    let mut s=db.prepare("SELECT p.id,p.title,p.description,p.kind,p.start_date,p.end_date,COUNT(t.id),COALESCE(SUM(CASE WHEN t.status='done' THEN 1 ELSE 0 END),0) FROM plans p LEFT JOIN tasks t ON t.plan_id=p.id AND t.deleted_at IS NULL WHERE p.deleted_at IS NULL GROUP BY p.id ORDER BY p.sort_order,p.created_at DESC").map_err(|e|e.to_string())?;
    let rows=s.query_map([],|r| Ok(json!({"id":r.get::<_,String>(0)?,"title":r.get::<_,String>(1)?,"description":r.get::<_,String>(2)?,"kind":r.get::<_,String>(3)?,"startDate":r.get::<_,Option<String>>(4)?,"endDate":r.get::<_,Option<String>>(5)?,"total":r.get::<_,i64>(6)?,"done":r.get::<_,i64>(7)?}))).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_plan(app: tauri::AppHandle, input: PlanInput) -> Result<String, String> {
    let db = open_database(&app)?;
    let id = new_id(&db)?;
    let kind = input.kind.unwrap_or_else(|| "study".into());
    db.execute("INSERT INTO plans(id,title,description,kind,start_date,end_date,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,datetime('now'),datetime('now'))",params![id,input.title.trim(),input.description.unwrap_or_default(),kind,input.start_date,input.end_date]).map_err(|e|e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn list_stages(app: tauri::AppHandle, plan_id: String) -> Result<Vec<Value>, String> {
    let db = open_database(&app)?;
    let mut s=db.prepare("SELECT s.id,s.title,s.description,COUNT(t.id),COALESCE(SUM(CASE WHEN t.status='done' THEN 1 ELSE 0 END),0) FROM stages s LEFT JOIN tasks t ON t.stage_id=s.id AND t.deleted_at IS NULL WHERE s.plan_id=?1 AND s.deleted_at IS NULL GROUP BY s.id ORDER BY s.sort_order,s.created_at").map_err(|e|e.to_string())?;
    let rows=s.query_map([plan_id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"title":r.get::<_,String>(1)?,"description":r.get::<_,String>(2)?,"total":r.get::<_,i64>(3)?,"done":r.get::<_,i64>(4)?}))).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_stage(app: tauri::AppHandle, plan_id: String, title: String) -> Result<String, String> {
    let db = open_database(&app)?;
    let id = new_id(&db)?;
    db.execute("INSERT INTO stages(id,plan_id,title,created_at,updated_at) VALUES(?1,?2,?3,datetime('now'),datetime('now'))",params![id,plan_id,title.trim()]).map_err(|e|e.to_string())?;
    Ok(id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentInput {
    title: String,
    kind: Option<String>,
    parent_id: Option<String>,
}

#[tauri::command]
fn list_documents(app: tauri::AppHandle) -> Result<Vec<Value>, String> {
    let db = open_database(&app)?;
    let mut s=db.prepare("SELECT id,parent_id,kind,title,relative_path,updated_at FROM documents WHERE deleted_at IS NULL ORDER BY kind='folder' DESC,sort_order,title COLLATE NOCASE").map_err(|e|e.to_string())?;
    let rows=s.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"parentId":r.get::<_,Option<String>>(1)?,"kind":r.get::<_,String>(2)?,"title":r.get::<_,String>(3)?,"relativePath":r.get::<_,String>(4)?,"updatedAt":r.get::<_,String>(5)?}))).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_document(app: tauri::AppHandle, input: DocumentInput) -> Result<String, String> {
    let db = open_database(&app)?;
    let id = new_id(&db)?;
    let kind = input.kind.unwrap_or_else(|| "note".into());
    let parent_path: Option<String> = if let Some(ref p) = input.parent_id {
        db.query_row(
            "SELECT relative_path FROM documents WHERE id=?1",
            [p],
            |r| r.get(0),
        )
        .ok()
    } else {
        None
    };
    let name = safe_name(&input.title);
    let relative = match (parent_path, kind.as_str()) {
        (Some(p), "folder") => format!("{p}/{name}"),
        (Some(p), _) => format!("{p}/{name}.md"),
        (None, "folder") => name.clone(),
        (None, _) => format!("{name}.md"),
    };
    let root = notes_root(&app)?;
    let path = root.join(&relative);
    if kind == "folder" {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?
    } else {
        atomic_write(&path, format!("# {}\n\n", input.title.trim()).as_bytes())?
    }
    db.execute("INSERT INTO documents(id,parent_id,kind,title,relative_path,file_size,sync_state,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,0,'synced',datetime('now'),datetime('now'))",params![id,input.parent_id,kind,input.title.trim(),relative]).map_err(|e|e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn read_document(app: tauri::AppHandle, id: String) -> Result<Value, String> {
    let db = open_database(&app)?;
    let (title, relative): (String, String) = db
        .query_row(
            "SELECT title,relative_path FROM documents WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let content =
        fs::read_to_string(notes_root(&app)?.join(&relative)).map_err(|e| e.to_string())?;
    Ok(json!({"title":title,"relativePath":relative,"content":content}))
}

#[tauri::command]
fn save_document(app: tauri::AppHandle, id: String, content: String) -> Result<(), String> {
    let db = open_database(&app)?;
    let relative: String = db
        .query_row(
            "SELECT relative_path FROM documents WHERE id=?1 AND deleted_at IS NULL",
            [&id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    atomic_write(&notes_root(&app)?.join(relative), content.as_bytes())?;
    db.execute("UPDATE documents SET file_size=?2,updated_at=datetime('now'),sync_state='synced' WHERE id=?1",params![id,content.len() as i64]).map_err(|e|e.to_string())?;
    Ok(())
}

#[tauri::command]
fn save_note_image(
    app: tauri::AppHandle,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<Value, String> {
    if bytes.is_empty() || bytes.len() > 25 * 1024 * 1024 {
        return Err("图片为空或超过 25MB".into());
    }
    let extension = Path::new(&file_name)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    if !["png", "jpg", "jpeg", "gif", "webp"].contains(&extension.as_str()) {
        return Err("不支持的图片格式".into());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let name = format!("image-{stamp}.{extension}");
    let absolute = data_root(&app)?.join("assets/images").join(&name);
    atomic_write(&absolute, &bytes)?;
    Ok(
        json!({"markdownPath":format!("../assets/images/{name}"),"absolutePath":absolute.to_string_lossy()}),
    )
}

#[tauri::command]
fn delete_document(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let db = open_database(&app)?;
    db.execute(
        "UPDATE documents SET deleted_at=datetime('now'),updated_at=datetime('now') WHERE id=?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn daily_note(app: tauri::AppHandle, date: String) -> Result<Value, String> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err("日期格式无效".into());
    }
    let relative = format!("{}/{}/{}.md", parts[0], parts[1], date);
    let path = data_root(&app)?.join("daily").join(&relative);
    if !path.exists() {
        atomic_write(&path,format!("# {date}\n\n## 今日计划\n\n- [ ] \n\n## 今日学习\n\n## 今天学到了什么\n\n## 遇到的问题\n\n## 今日总结\n\n## 明日计划\n").as_bytes())?
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(json!({"date":date,"relativePath":relative,"content":content}))
}

#[tauri::command]
fn save_daily_note(app: tauri::AppHandle, date: String, content: String) -> Result<(), String> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err("日期格式无效".into());
    }
    atomic_write(
        &data_root(&app)?
            .join("daily")
            .join(parts[0])
            .join(parts[1])
            .join(format!("{date}.md")),
        content.as_bytes(),
    )
}

#[tauri::command]
fn get_checkin(app: tauri::AppHandle, date: String) -> Result<Value, String> {
    let db = open_database(&app)?;
    let row=db.query_row("SELECT mood,summary_markdown,completed_tasks,total_tasks,study_minutes FROM checkins WHERE checkin_date=?1 AND deleted_at IS NULL",[date],|r|Ok(json!({"mood":r.get::<_,Option<i64>>(0)?,"summary":r.get::<_,String>(1)?,"completedTasks":r.get::<_,i64>(2)?,"totalTasks":r.get::<_,i64>(3)?,"studyMinutes":r.get::<_,i64>(4)?}))).ok();
    Ok(row.unwrap_or(json!(null)))
}

#[tauri::command]
fn save_checkin(
    app: tauri::AppHandle,
    date: String,
    mood: i64,
    summary: String,
) -> Result<(), String> {
    let db = open_database(&app)?;
    let (done,total):(i64,i64)=db.query_row("SELECT SUM(CASE WHEN status='done' THEN 1 ELSE 0 END),COUNT(*) FROM tasks WHERE planned_date=?1 AND deleted_at IS NULL",[&date],|r|Ok((r.get::<_,Option<i64>>(0)?.unwrap_or(0),r.get(1)?))).map_err(|e|e.to_string())?;
    let mins:i64=db.query_row("SELECT COALESCE(SUM(duration_minutes),0) FROM study_sessions WHERE date(started_at)=?1 AND deleted_at IS NULL",[&date],|r|r.get(0)).map_err(|e|e.to_string())?;
    let id = new_id(&db)?;
    db.execute("INSERT INTO checkins(id,checkin_date,mood,summary_markdown,completed_tasks,total_tasks,study_minutes,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,datetime('now'),datetime('now')) ON CONFLICT(checkin_date) DO UPDATE SET mood=excluded.mood,summary_markdown=excluded.summary_markdown,completed_tasks=excluded.completed_tasks,total_tasks=excluded.total_tasks,study_minutes=excluded.study_minutes,updated_at=datetime('now'),deleted_at=NULL",params![id,date,mood,summary,done,total,mins]).map_err(|e|e.to_string())?;
    Ok(())
}

#[tauri::command]
fn analytics(app: tauri::AppHandle) -> Result<Value, String> {
    let db = open_database(&app)?;
    let today:i64=db.query_row("SELECT COALESCE(SUM(duration_minutes),0) FROM study_sessions WHERE date(started_at)=date('now','localtime') AND deleted_at IS NULL",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    let week:i64=db.query_row("SELECT COALESCE(SUM(duration_minutes),0) FROM study_sessions WHERE date(started_at)>=date('now','localtime','-6 days') AND deleted_at IS NULL",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    let month:i64=db.query_row("SELECT COALESCE(SUM(duration_minutes),0) FROM study_sessions WHERE strftime('%Y-%m',started_at)=strftime('%Y-%m','now','localtime') AND deleted_at IS NULL",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    let mut s=db.prepare("WITH RECURSIVE d(x) AS (VALUES(date('now','localtime','-6 days')) UNION ALL SELECT date(x,'+1 day') FROM d WHERE x<date('now','localtime')) SELECT x,COALESCE((SELECT SUM(duration_minutes) FROM study_sessions WHERE date(started_at)=x AND deleted_at IS NULL),0),(SELECT COUNT(*) FROM tasks WHERE planned_date=x AND status='done' AND deleted_at IS NULL) FROM d").map_err(|e|e.to_string())?;
    let trend=s.query_map([],|r|Ok(json!({"date":r.get::<_,String>(0)?,"minutes":r.get::<_,i64>(1)?,"tasks":r.get::<_,i64>(2)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let days:i64=db.query_row("SELECT COUNT(*) FROM checkins WHERE strftime('%Y',checkin_date)=strftime('%Y','now','localtime') AND deleted_at IS NULL",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    Ok(
        json!({"todayMinutes":today,"weekMinutes":week,"monthMinutes":month,"studyDays":days,"trend":trend}),
    )
}

#[tauri::command]
fn calendar_month(app: tauri::AppHandle, month: String) -> Result<Vec<Value>, String> {
    let db = open_database(&app)?;
    let mut s=db.prepare("SELECT d,COUNT(t.id),COALESCE(SUM(CASE WHEN t.status='done' THEN 1 ELSE 0 END),0),COALESCE((SELECT SUM(duration_minutes) FROM study_sessions WHERE date(started_at)=d AND deleted_at IS NULL),0),EXISTS(SELECT 1 FROM checkins WHERE checkin_date=d AND deleted_at IS NULL) FROM (SELECT planned_date d FROM tasks WHERE planned_date LIKE ?1||'%' UNION SELECT checkin_date FROM checkins WHERE checkin_date LIKE ?1||'%' UNION SELECT date(started_at) FROM study_sessions WHERE date(started_at) LIKE ?1||'%') x LEFT JOIN tasks t ON t.planned_date=d AND t.deleted_at IS NULL GROUP BY d ORDER BY d").map_err(|e|e.to_string())?;
    let rows=s.query_map([month],|r|Ok(json!({"date":r.get::<_,String>(0)?,"total":r.get::<_,i64>(1)?,"done":r.get::<_,i64>(2)?,"minutes":r.get::<_,i64>(3)?,"checkedIn":r.get::<_,bool>(4)?}))).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn global_search(app: tauri::AppHandle, query: String) -> Result<Vec<Value>, String> {
    let db = open_database(&app)?;
    let needle = query.trim().to_lowercase();
    let like = format!("%{}%", query.trim());
    let mut out = Vec::new();
    let mut s=db.prepare("SELECT 'task',id,title,description FROM tasks WHERE deleted_at IS NULL AND (title LIKE ?1 OR description LIKE ?1) UNION ALL SELECT 'plan',id,title,description FROM plans WHERE deleted_at IS NULL AND (title LIKE ?1 OR description LIKE ?1) UNION ALL SELECT 'note',id,title,relative_path FROM documents WHERE deleted_at IS NULL AND kind!='folder' AND title LIKE ?1 LIMIT 30").map_err(|e|e.to_string())?;
    for row in s.query_map([like],|r|Ok(json!({"type":r.get::<_,String>(0)?,"id":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"subtitle":r.get::<_,String>(3)?}))).map_err(|e|e.to_string())?{out.push(row.map_err(|e|e.to_string())?)}
    drop(s);
    let existing = out
        .iter()
        .filter_map(|x| x.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut notes = db
        .prepare(
            "SELECT id,title,relative_path FROM documents WHERE deleted_at IS NULL AND kind='note'",
        )
        .map_err(|e| e.to_string())?;
    let note_rows = notes
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in note_rows {
        let (id, title, path) = row.map_err(|e| e.to_string())?;
        if existing.contains(&id) {
            continue;
        }
        let body = fs::read_to_string(notes_root(&app)?.join(&path)).unwrap_or_default();
        if body.to_lowercase().contains(&needle) {
            out.push(json!({"type":"note","id":id,"title":title,"subtitle":path}));
        }
        if out.len() >= 30 {
            break;
        }
    }
    Ok(out)
}

#[tauri::command]
fn trash_items(app: tauri::AppHandle) -> Result<Vec<Value>, String> {
    let db = open_database(&app)?;
    let mut s=db.prepare("SELECT 'task',id,title,deleted_at FROM tasks WHERE deleted_at IS NOT NULL UNION ALL SELECT 'plan',id,title,deleted_at FROM plans WHERE deleted_at IS NOT NULL UNION ALL SELECT 'note',id,title,deleted_at FROM documents WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC").map_err(|e|e.to_string())?;
    let rows=s.query_map([],|r|Ok(json!({"type":r.get::<_,String>(0)?,"id":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"deletedAt":r.get::<_,String>(3)?}))).map_err(|e|e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_item(app: tauri::AppHandle, item_type: String, id: String) -> Result<(), String> {
    let table = match item_type.as_str() {
        "task" => "tasks",
        "plan" => "plans",
        "note" => "documents",
        _ => return Err("类型无效".into()),
    };
    let db = open_database(&app)?;
    db.execute(
        &format!("UPDATE {table} SET deleted_at=NULL,updated_at=datetime('now') WHERE id=?1"),
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn create_backup(app: tauri::AppHandle) -> Result<String, String> {
    let root = data_root(&app)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs()
        .to_string();
    let target = root.join("backups").join(&stamp);
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let snapshot = target.join("planner.db");
    let db = open_database(&app)?;
    db.execute("VACUUM INTO ?1", [snapshot.to_string_lossy().into_owned()])
        .map_err(|e| e.to_string())?;
    drop(db);
    copy_tree(&notes_root(&app)?, &target.join("notes"))?;
    for folder in ["daily", "assets"] {
        copy_tree(&root.join(folder), &target.join(folder))?;
    }
    let retain = open_database(&app)?
        .query_row(
            "SELECT value_json FROM settings WHERE key='backup_retain'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "7".into())
        .parse::<usize>()
        .unwrap_or(7)
        .clamp(1, 50);
    let mut backups = fs::read_dir(root.join("backups"))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|x| x.path().is_dir())
        .collect::<Vec<_>>();
    backups.sort_by_key(|x| x.file_name());
    let remove_count = backups.len().saturating_sub(retain);
    for old in backups.into_iter().take(remove_count) {
        fs::remove_dir_all(old.path()).map_err(|e| e.to_string())?;
    }
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
fn get_preferences(app: tauri::AppHandle) -> Result<Value, String> {
    let db = open_database(&app)?;
    let mode = db
        .query_row(
            "SELECT value_json FROM settings WHERE key='backup_mode'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "daily".into());
    let retain = db
        .query_row(
            "SELECT value_json FROM settings WHERE key='backup_retain'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "7".into())
        .parse::<usize>()
        .unwrap_or(7);
    Ok(
        json!({"backupMode":mode,"backupRetain":retain,"notesRoot":notes_root(&app)?.to_string_lossy()}),
    )
}

#[tauri::command]
fn set_notes_root(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let target = PathBuf::from(path.trim());
    if !target.is_absolute() {
        return Err("请输入完整的绝对路径".into());
    }
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let current = notes_root(&app)?;
    if current != target {
        copy_tree(&current, &target)?;
    }
    let db = open_database(&app)?;
    db.execute("INSERT INTO settings(key,value_json) VALUES('notes_root',?1) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=datetime('now')", [target.to_string_lossy().into_owned()]).map_err(|e|e.to_string())?;
    Ok(())
}

fn setting(db: &Connection, key: &str) -> Option<String> {
    db.query_row("SELECT value_json FROM settings WHERE key=?1", [key], |r| {
        r.get(0)
    })
    .ok()
}
fn put_setting(db: &Connection, key: &str, value: &str) -> Result<(), String> {
    db.execute("INSERT INTO settings(key,value_json) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=datetime('now')",params![key,value]).map_err(|e|e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_yuque_config(app: tauri::AppHandle) -> Result<Value, String> {
    let db = open_database(&app)?;
    Ok(
        json!({"host":setting(&db,"yuque_host").unwrap_or_else(||"https://www.yuque.com".into()),"login":setting(&db,"yuque_login").unwrap_or_default(),"repo":setting(&db,"yuque_repo").unwrap_or_default(),"tokenConfigured":setting(&db,"yuque_token").is_some()}),
    )
}

#[tauri::command]
fn save_yuque_config(
    app: tauri::AppHandle,
    host: String,
    login: String,
    repo: String,
    token: String,
) -> Result<(), String> {
    let db = open_database(&app)?;
    put_setting(&db, "yuque_host", host.trim_end_matches('/'))?;
    put_setting(&db, "yuque_login", login.trim())?;
    put_setting(&db, "yuque_repo", repo.trim())?;
    if !token.trim().is_empty() {
        put_setting(&db, "yuque_token", token.trim())?
    }
    Ok(())
}

fn yuque_parts(app: &tauri::AppHandle) -> Result<(String, String, String, String), String> {
    let db = open_database(app)?;
    let host = setting(&db, "yuque_host").unwrap_or_else(|| "https://www.yuque.com".into());
    let login = setting(&db, "yuque_login").unwrap_or_default();
    let repo = setting(&db, "yuque_repo").unwrap_or_default();
    let token = setting(&db, "yuque_token").ok_or("请先配置语雀 Token")?;
    if login.is_empty() || repo.is_empty() {
        return Err("请填写语雀账号/空间和知识库路径".into());
    }
    Ok((host, login, repo, token))
}

#[tauri::command]
fn yuque_list_docs(app: tauri::AppHandle) -> Result<Vec<Value>, String> {
    let (host, login, repo, token) = yuque_parts(&app)?;
    let base = if host.ends_with("/api/v2") {
        host
    } else {
        format!("{host}/api/v2")
    };
    let url = format!("{base}/repos/{login}/{repo}/docs?limit=100");
    let response = reqwest::blocking::Client::new()
        .get(url)
        .header("X-Auth-Token", token)
        .header("User-Agent", "Chaomu/0.1")
        .send()
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("语雀连接失败：{}", response.status()));
    }
    let value: Value = response.json().map_err(|e| e.to_string())?;
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(data.into_iter().map(|x|json!({"id":x.get("id"),"title":x.get("title"),"slug":x.get("slug"),"updatedAt":x.get("content_updated_at").or_else(||x.get("updated_at"))})).collect())
}

#[tauri::command]
fn yuque_create_doc(app: tauri::AppHandle, title: String, body: String) -> Result<Value, String> {
    let (host, login, repo, token) = yuque_parts(&app)?;
    let base = if host.ends_with("/api/v2") {
        host.clone()
    } else {
        format!("{host}/api/v2")
    };
    let url = format!("{base}/repos/{login}/{repo}/docs");
    let response = reqwest::blocking::Client::new()
        .post(url)
        .header("X-Auth-Token", token)
        .header("User-Agent", "Chaomu/0.1")
        .json(&json!({"title":title,"format":"markdown","body":body}))
        .send()
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("创建语雀文档失败：{}", response.status()));
    }
    let value: Value = response.json().map_err(|e| e.to_string())?;
    Ok(
        json!({"url":format!("{host}/{login}/{repo}/{}",value.pointer("/data/slug").and_then(Value::as_str).unwrap_or(""))}),
    )
}

#[tauri::command]
fn save_preferences(
    app: tauri::AppHandle,
    backup_mode: String,
    backup_retain: usize,
) -> Result<(), String> {
    if !["off", "daily", "weekly"].contains(&backup_mode.as_str()) {
        return Err("无效的备份频率".into());
    }
    let db = open_database(&app)?;
    db.execute("INSERT INTO settings(key,value_json) VALUES('backup_mode',?1) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=datetime('now')", [&backup_mode]).map_err(|e|e.to_string())?;
    db.execute("INSERT INTO settings(key,value_json) VALUES('backup_retain',?1) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=datetime('now')", [backup_retain.clamp(1,50).to_string()]).map_err(|e|e.to_string())?;
    Ok(())
}

fn maybe_auto_backup(app: &tauri::AppHandle) -> Result<(), String> {
    let db = open_database(app)?;
    let mode = db
        .query_row(
            "SELECT value_json FROM settings WHERE key='backup_mode'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "daily".into());
    if mode == "off" {
        return Ok(());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let last = db
        .query_row(
            "SELECT value_json FROM settings WHERE key='last_auto_backup'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default()
        .parse::<u64>()
        .unwrap_or(0);
    let interval = if mode == "weekly" { 7 * 86400 } else { 86400 };
    if now.saturating_sub(last) < interval {
        return Ok(());
    }
    drop(db);
    create_backup(app.clone())?;
    let db = open_database(app)?;
    db.execute("INSERT INTO settings(key,value_json) VALUES('last_auto_backup',?1) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=datetime('now')", [now.to_string()]).map_err(|e|e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            initialize_storage(app.handle()).map_err(std::io::Error::other)?;
            let _ = maybe_auto_backup(app.handle());
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            list_tasks,
            create_task,
            set_task_status,
            delete_task,
            update_task,
            reorder_tasks,
            set_task_important,
            list_task_daily_items,
            today_daily_items,
            add_task_daily_item,
            set_daily_item_done,
            list_knowledge_categories,
            create_knowledge_category,
            rename_knowledge_category,
            delete_knowledge_category,
            list_knowledge_files,
            upload_knowledge_file,
            open_reminders,
            list_projects,
            create_project,
            rename_project,
            delete_project,
            task_daily_progress,
            project_tasks,
            record_study,
            list_plans,
            create_plan,
            list_stages,
            create_stage,
            list_documents,
            create_document,
            read_document,
            save_document,
            save_note_image,
            delete_document,
            daily_note,
            save_daily_note,
            get_checkin,
            save_checkin,
            analytics,
            calendar_month,
            global_search,
            trash_items,
            restore_item,
            create_backup,
            get_preferences,
            save_preferences,
            set_notes_root,
            get_yuque_config,
            save_yuque_config,
            yuque_list_docs,
            yuque_create_doc
        ])
        .run(tauri::generate_context!())
        .expect("error while running 朝暮");
}
