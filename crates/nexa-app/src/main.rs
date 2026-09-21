#![deny(unsafe_code)]

mod docs_session;
mod platform;
mod settings_store;

use docs_session::{DocsSession, DocsSessionError};
use nexa_core::{AppCommand, AppLanguage, AppPage, AppSettings, AppState, EditorKind};
use platform::PlatformInfo;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    time::Instant,
};

slint::include_modules!();

type SharedState = Rc<RefCell<AppState>>;
type SharedDocs = Rc<RefCell<Option<DocsSession>>>;

fn main() -> Result<(), slint::PlatformError> {
    let startup = Instant::now();
    let ui = AppWindow::new()?;
    let settings_path = platform::settings_path();
    let settings = load_settings(settings_path.as_deref());
    let state = Rc::new(RefCell::new(AppState::with_settings(settings)));
    let docs = Rc::new(RefCell::new(None));

    configure_static_diagnostics(&ui);

    bind_navigation(
        &ui,
        Rc::clone(&state),
        Rc::clone(&docs),
        settings_path.clone(),
    );
    bind_editor_actions(
        &ui,
        Rc::clone(&state),
        Rc::clone(&docs),
        settings_path.clone(),
    );
    bind_docs_actions(&ui, Rc::clone(&state), Rc::clone(&docs));
    bind_settings(&ui, Rc::clone(&state), settings_path.clone());

    if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) {
        if is_docx_path(&path) {
            open_docs_path(&state, &docs, &ui.as_weak(), path);
        } else {
            update_state(
                &state,
                &ui.as_weak(),
                settings_path.as_deref(),
                AppCommand::OpenFile(path),
            );
        }
    } else {
        sync_ui(&state.borrow(), &ui);
        sync_docs_ui(docs.borrow().as_ref(), &ui);
    }

    ui.set_shell_init_text(format!("{:.1} ms", startup.elapsed().as_secs_f64() * 1000.0).into());
    ui.run()
}

fn load_settings(path: Option<&Path>) -> AppSettings {
    path.and_then(|path| settings_store::load(path).ok())
        .unwrap_or_default()
}

fn configure_static_diagnostics(ui: &AppWindow) {
    let info = PlatformInfo::current();
    ui.set_platform_text(info.os.into());
    ui.set_architecture_text(info.architecture.into());
    ui.set_build_text(format!("{} · v{}", info.build_profile, info.version).into());
    ui.set_renderer_text(info.renderer.into());
}

fn bind_navigation(
    ui: &AppWindow,
    state: SharedState,
    docs: SharedDocs,
    settings_path: Option<PathBuf>,
) {
    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        let settings_path = settings_path.clone();
        ui.on_show_home(move || {
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::Navigate(AppPage::Home),
            );
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_show_docs(move || {
            if docs.borrow().is_none() {
                *docs.borrow_mut() = Some(DocsSession::blank());
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_docs_path_input("".into());
                }
            }

            {
                let mut state = state.borrow_mut();
                state.apply(AppCommand::New(EditorKind::Docs));
                if let Some(session) = docs.borrow().as_ref() {
                    state.set_status(format!("Editing {}", session.title()));
                }
            }

            if let Some(ui) = ui_weak.upgrade() {
                sync_ui(&state.borrow(), &ui);
                sync_docs_ui(docs.borrow().as_ref(), &ui);
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        let settings_path = settings_path.clone();
        ui.on_show_diagnostics(move || {
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::Navigate(AppPage::Diagnostics),
            );
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_show_settings(move || {
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::Navigate(AppPage::Settings),
            );
        });
    }
}

fn bind_editor_actions(
    ui: &AppWindow,
    state: SharedState,
    docs: SharedDocs,
    settings_path: Option<PathBuf>,
) {
    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        let settings_path = settings_path.clone();
        ui.on_create_docs(move || {
            *docs.borrow_mut() = Some(DocsSession::blank());
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_docs_path_input("".into());
                ui.set_docs_search_query("".into());
                ui.set_docs_replace_text("".into());
            }
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::New(EditorKind::Docs),
            );
            if let Some(ui) = ui_weak.upgrade() {
                sync_docs_ui(docs.borrow().as_ref(), &ui);
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        let settings_path = settings_path.clone();
        ui.on_create_sheets(move || {
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::New(EditorKind::Sheets),
            );
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_create_slides(move || {
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::New(EditorKind::Slides),
            );
        });
    }
}

fn bind_docs_actions(ui: &AppWindow, state: SharedState, docs: SharedDocs) {
    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_open_path(move |path| {
            open_docs_path(
                &state,
                &docs,
                &ui_weak,
                PathBuf::from(path.trim().to_string()),
            );
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_save(move || {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                session.save()?;
                Ok(format!("Saved {}", session.title()))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_save_as(move |path| {
            let path = PathBuf::from(path.trim().to_string());
            apply_docs_operation(&state, &docs, &ui_weak, move |session| {
                session.save_as(path)?;
                Ok(format!("Saved {}", session.title()))
            });
            if let Some(ui) = ui_weak.upgrade()
                && let Some(session) = docs.borrow().as_ref()
            {
                ui.set_docs_path_input(session.path_text().into());
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_edit_paragraph(move |text| {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                session.set_current_paragraph_text(text.as_str())?;
                Ok(format!(
                    "Editing paragraph {}",
                    session.current_paragraph_index() + 1
                ))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_previous_paragraph(move || {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                session.previous_paragraph();
                Ok(format!(
                    "Paragraph {}",
                    session.current_paragraph_index() + 1
                ))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_next_paragraph(move || {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                session.next_paragraph();
                Ok(format!(
                    "Paragraph {}",
                    session.current_paragraph_index() + 1
                ))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_insert_paragraph(move || {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                session.insert_paragraph_after_current()?;
                Ok(format!(
                    "Inserted paragraph {}",
                    session.current_paragraph_index() + 1
                ))
            });
        });
    }

    bind_format_action(ui, Rc::clone(&state), Rc::clone(&docs), FormatAction::Bold);
    bind_format_action(
        ui,
        Rc::clone(&state),
        Rc::clone(&docs),
        FormatAction::Italic,
    );
    bind_format_action(
        ui,
        Rc::clone(&state),
        Rc::clone(&docs),
        FormatAction::Underline,
    );

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_undo(move || {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                let changed = session.undo()?;
                Ok(if changed {
                    "Undo".to_owned()
                } else {
                    "Nothing to undo".to_owned()
                })
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_redo(move || {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                let changed = session.redo()?;
                Ok(if changed {
                    "Redo".to_owned()
                } else {
                    "Nothing to redo".to_owned()
                })
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_search(move |query| {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                if query.trim().is_empty() {
                    return Ok("Enter text to search".to_owned());
                }
                let count = session.search_count(query.as_str())?;
                Ok(format!("{count} match(es)"))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let ui_weak = ui.as_weak();
        ui.on_docs_replace_all(move |query, replacement| {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                if query.trim().is_empty() {
                    return Ok("Enter text to replace".to_owned());
                }
                let count = session.replace_all(query.as_str(), replacement.as_str())?;
                Ok(format!("Replaced {count} match(es)"))
            });
        });
    }
}

#[derive(Clone, Copy)]
enum FormatAction {
    Bold,
    Italic,
    Underline,
}

fn bind_format_action(ui: &AppWindow, state: SharedState, docs: SharedDocs, action: FormatAction) {
    let bind = move |ui_weak: slint::Weak<AppWindow>| {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        move || {
            apply_docs_operation(&state, &docs, &ui_weak, |session| {
                match action {
                    FormatAction::Bold => session.toggle_bold()?,
                    FormatAction::Italic => session.toggle_italic()?,
                    FormatAction::Underline => session.toggle_underline()?,
                }
                Ok("Formatting updated".to_owned())
            });
        }
    };

    match action {
        FormatAction::Bold => ui.on_docs_toggle_bold(bind(ui.as_weak())),
        FormatAction::Italic => ui.on_docs_toggle_italic(bind(ui.as_weak())),
        FormatAction::Underline => ui.on_docs_toggle_underline(bind(ui.as_weak())),
    }
}

fn bind_settings(ui: &AppWindow, state: SharedState, settings_path: Option<PathBuf>) {
    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        let settings_path = settings_path.clone();
        ui.on_toggle_status_bar(move || {
            let next = !state.borrow().settings().show_status_bar();
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::SetShowStatusBar(next),
            );
        });
    }

    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        let settings_path = settings_path.clone();
        ui.on_toggle_compact_navigation(move || {
            let next = !state.borrow().settings().compact_navigation();
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::SetCompactNavigation(next),
            );
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_set_language(move |value| {
            let language = match value {
                1 => AppLanguage::SimplifiedChinese,
                2 => AppLanguage::English,
                _ => AppLanguage::System,
            };
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::SetLanguage(language),
            );
        });
    }
}

fn open_docs_path(
    state: &SharedState,
    docs: &SharedDocs,
    ui: &slint::Weak<AppWindow>,
    path: PathBuf,
) {
    match DocsSession::open(path.clone()) {
        Ok(session) => {
            *docs.borrow_mut() = Some(session);
            {
                let mut state = state.borrow_mut();
                state.apply(AppCommand::OpenFile(path));
                if let Some(session) = docs.borrow().as_ref() {
                    state.set_status(format!("Opened {}", session.title()));
                }
            }
            if let Some(ui) = ui.upgrade() {
                if let Some(session) = docs.borrow().as_ref() {
                    ui.set_docs_path_input(session.path_text().into());
                }
                sync_ui(&state.borrow(), &ui);
                sync_docs_ui(docs.borrow().as_ref(), &ui);
            }
        }
        Err(error) => {
            state
                .borrow_mut()
                .set_status(format!("Open failed: {error}"));
            if let Some(ui) = ui.upgrade() {
                sync_ui(&state.borrow(), &ui);
            }
        }
    }
}

fn apply_docs_operation(
    state: &SharedState,
    docs: &SharedDocs,
    ui: &slint::Weak<AppWindow>,
    operation: impl FnOnce(&mut DocsSession) -> Result<String, DocsSessionError>,
) {
    let result = {
        let mut docs = docs.borrow_mut();
        match docs.as_mut() {
            Some(session) => operation(session),
            None => Ok("No Docs document is open".to_owned()),
        }
    };

    match result {
        Ok(status) => state.borrow_mut().set_status(status),
        Err(error) => state
            .borrow_mut()
            .set_status(format!("Docs command failed: {error}")),
    }

    if let Some(ui) = ui.upgrade() {
        sync_ui(&state.borrow(), &ui);
        sync_docs_ui(docs.borrow().as_ref(), &ui);
    }
}

fn update_state(
    state: &SharedState,
    ui: &slint::Weak<AppWindow>,
    settings_path: Option<&Path>,
    command: AppCommand,
) {
    let persist_settings = matches!(
        &command,
        AppCommand::SetShowStatusBar(_)
            | AppCommand::SetCompactNavigation(_)
            | AppCommand::SetLanguage(_)
    );

    let persistence_error = {
        let mut state = state.borrow_mut();
        state.apply(command);

        if persist_settings {
            persist_settings_if_available(settings_path, state.settings())
        } else {
            None
        }
    };

    if let Some(ui) = ui.upgrade() {
        sync_ui(&state.borrow(), &ui);

        if let Some(error) = persistence_error {
            eprintln!("failed to persist Nexa settings: {error}");
            ui.set_status_text(
                if ui.get_is_chinese() {
                    "设置已在本次会话生效，但保存到本机失败"
                } else {
                    "Setting changed for this session; saving failed"
                }
                .into(),
            );
        }
    }
}

fn persist_settings_if_available(
    settings_path: Option<&Path>,
    settings: &AppSettings,
) -> Option<std::io::Error> {
    let path = settings_path?;
    settings_store::save(path, settings).err()
}

fn sync_ui(state: &AppState, ui: &AppWindow) {
    let page = if state.active_editor() == Some(EditorKind::Docs) {
        3
    } else {
        match state.page() {
            AppPage::Home => 0,
            AppPage::Diagnostics => 1,
            AppPage::Settings => 2,
        }
    };
    let language = state.settings().language();
    let is_chinese = resolved_is_chinese(language);

    ui.set_page(page);
    ui.set_is_chinese(is_chinese);
    ui.set_language_mode(match language {
        AppLanguage::System => 0,
        AppLanguage::SimplifiedChinese => 1,
        AppLanguage::English => 2,
    });
    ui.set_status_text(localize_status(state.status(), is_chinese).into());
    ui.set_show_status_bar(state.settings().show_status_bar());
    ui.set_compact_navigation(state.settings().compact_navigation());
}

fn sync_docs_ui(session: Option<&DocsSession>, ui: &AppWindow) {
    let is_chinese = ui.get_is_chinese();
    let Some(session) = session else {
        ui.set_docs_title("Docs".into());
        ui.set_docs_current_path(if is_chinese {
            "未打开文档".into()
        } else {
            "No document open".into()
        });
        ui.set_docs_paragraph_text("".into());
        ui.set_docs_paragraph_meta(if is_chinese {
            "第 0 段 / 共 0 段".into()
        } else {
            "Paragraph 0 of 0".into()
        });
        ui.set_docs_page_meta(if is_chinese {
            "0 页".into()
        } else {
            "0 pages".into()
        });
        ui.set_docs_dirty_meta("".into());
        ui.set_docs_compatibility_text("".into());
        ui.set_docs_bold(false);
        ui.set_docs_italic(false);
        ui.set_docs_underline(false);
        ui.set_docs_can_save(false);
        return;
    };

    ui.set_docs_title(session.title().into());
    ui.set_docs_current_path(if session.path_text().is_empty() {
        if is_chinese {
            "尚未保存".into()
        } else {
            "Not saved yet".into()
        }
    } else {
        session.path_text().into()
    });
    ui.set_docs_paragraph_text(session.current_paragraph_text().into());
    ui.set_docs_paragraph_meta(
        if is_chinese {
            format!(
                "第 {} 段 / 共 {} 段",
                session.current_paragraph_index() + 1,
                session.paragraph_count()
            )
        } else {
            format!(
                "Paragraph {} of {}",
                session.current_paragraph_index() + 1,
                session.paragraph_count()
            )
        }
        .into(),
    );
    ui.set_docs_page_meta(
        if is_chinese {
            format!("{} 页", session.page_count())
        } else {
            format!("{} page(s)", session.page_count())
        }
        .into(),
    );
    ui.set_docs_dirty_meta(
        if session.is_dirty() {
            if is_chinese {
                "有未保存更改"
            } else {
                "Unsaved changes"
            }
        } else if is_chinese {
            "已保存"
        } else {
            "Saved"
        }
        .into(),
    );
    ui.set_docs_compatibility_text(if session.can_save() {
        if is_chinese {
            "兼容性检查：可安全写入".into()
        } else {
            "Compatibility check: writable".into()
        }
    } else if is_chinese {
        format!(
            "已阻止保存 · 检测到 {} 个暂不支持的结构",
            session.compatibility_issue_count()
        )
        .into()
    } else {
        format!(
            "Save blocked · {} unsupported construct(s)",
            session.compatibility_issue_count()
        )
        .into()
    });
    ui.set_docs_bold(session.current_bold());
    ui.set_docs_italic(session.current_italic());
    ui.set_docs_underline(session.current_underline());
    ui.set_docs_can_save(session.can_save());
}

fn is_docx_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("docx"))
}

fn resolved_is_chinese(language: AppLanguage) -> bool {
    match language {
        AppLanguage::SimplifiedChinese => true,
        AppLanguage::English => false,
        AppLanguage::System => system_prefers_chinese(),
    }
}

fn system_prefers_chinese() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LC_CTYPE", "LANGUAGE", "LANG"]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok())
        .any(|value| {
            let normalized = value.to_ascii_lowercase();
            normalized.starts_with("zh") || normalized.contains(":zh") || normalized.contains("_zh")
        })
}

fn localize_status(status: &str, is_chinese: bool) -> String {
    if !is_chinese {
        return status.to_owned();
    }

    match status {
        "Native shell ready" => "原生工作区已就绪".to_owned(),
        "Home" => "首页".to_owned(),
        "Diagnostics" => "诊断".to_owned(),
        "Settings" => "设置".to_owned(),
        "New Docs document" => "已新建文档".to_owned(),
        "Sheets editor is planned for Phase 4" => "表格编辑器将在 Phase 4 实现".to_owned(),
        "Slides editor is planned for Phase 5" => "演示编辑器将在 Phase 5 实现".to_owned(),
        "Status bar shown" => "已显示状态栏".to_owned(),
        "Status bar hidden" => "已隐藏状态栏".to_owned(),
        "Compact navigation enabled" => "已启用紧凑导航".to_owned(),
        "Compact navigation disabled" => "已关闭紧凑导航".to_owned(),
        "Language preference updated" => "语言设置已更新".to_owned(),
        "Formatting updated" => "格式已更新".to_owned(),
        "Undo" => "已撤销".to_owned(),
        "Redo" => "已重做".to_owned(),
        "Nothing to undo" => "没有可撤销的操作".to_owned(),
        "Nothing to redo" => "没有可重做的操作".to_owned(),
        "Enter text to search" => "请输入要查找的内容".to_owned(),
        "Enter text to replace" => "请输入要替换的内容".to_owned(),
        "No Docs document is open" => "当前未打开文档".to_owned(),
        other if other.starts_with("Editing paragraph ") => {
            format!("正在编辑第 {} 段", &other["Editing paragraph ".len()..])
        }
        other if other.starts_with("Paragraph ") => {
            format!("第 {} 段", &other["Paragraph ".len()..])
        }
        other if other.starts_with("Inserted paragraph ") => {
            format!("已插入第 {} 段", &other["Inserted paragraph ".len()..])
        }
        other if other.starts_with("Editing ") => {
            format!("正在编辑 {}", &other["Editing ".len()..])
        }
        other if other.starts_with("Opened ") => {
            format!("已打开 {}", &other["Opened ".len()..])
        }
        other if other.starts_with("Saved ") => {
            format!("已保存 {}", &other["Saved ".len()..])
        }
        other if other.starts_with("Replaced ") && other.ends_with(" match(es)") => {
            let count = other
                .trim_start_matches("Replaced ")
                .trim_end_matches(" match(es)");
            format!("已替换 {count} 处")
        }
        other if other.ends_with(" match(es)") => {
            format!("找到 {} 处匹配", other.trim_end_matches(" match(es)"))
        }
        other if other.starts_with("Open failed: ") => {
            format!("打开失败：{}", &other["Open failed: ".len()..])
        }
        other if other.starts_with("Docs command failed: ") => {
            format!("文档操作失败：{}", &other["Docs command failed: ".len()..])
        }
        _ => status.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_status_localizes_static_and_dynamic_docs_feedback() {
        assert_eq!(
            localize_status("Native shell ready", true),
            "原生工作区已就绪"
        );
        assert_eq!(
            localize_status("Editing paragraph 3", true),
            "正在编辑第 3 段"
        );
        assert_eq!(localize_status("Replaced 4 match(es)", true), "已替换 4 处");
        assert_eq!(localize_status("7 match(es)", true), "找到 7 处匹配");
    }

    #[test]
    fn english_status_remains_stable() {
        assert_eq!(
            localize_status("Opened quarterly.docx", false),
            "Opened quarterly.docx"
        );
    }
}
