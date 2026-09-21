#![deny(unsafe_code)]

mod docs_session;
mod native_integration;
mod platform;
mod recent_store;
mod recovery_store;
mod settings_store;
mod sheets_session;
mod slides_session;

use docs_session::{DocsSession, DocsSessionError};
use nexa_core::{AppCommand, AppLanguage, AppPage, AppSettings, AppState, EditorKind};
use platform::PlatformInfo;
use recovery_store::RecoveryKind;
use sheets_session::{SheetRowData, SheetsSession, SheetsSessionError};
use slides_session::{SlideElementSummary, SlidesSession, SlidesSessionError};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    time::Instant,
};

slint::include_modules!();

type SharedState = Rc<RefCell<AppState>>;
type SharedDocs = Rc<RefCell<Option<DocsSession>>>;
type SharedSheets = Rc<RefCell<Option<SheetsSession>>>;
type SharedSlides = Rc<RefCell<Option<SlidesSession>>>;

fn main() -> Result<(), slint::PlatformError> {
    let startup = Instant::now();
    let ui = AppWindow::new()?;
    let settings_path = platform::settings_path();
    let recent_path = platform::recent_files_path();
    let recovery_directory = platform::recovery_directory();
    let settings = load_settings(settings_path.as_deref());
    let recent_files = recent_path
        .as_deref()
        .and_then(|path| recent_store::load(path).ok())
        .unwrap_or_default();
    let state = Rc::new(RefCell::new(AppState::with_settings_and_recent(
        settings,
        recent_files,
    )));
    let docs = Rc::new(RefCell::new(None));
    let sheets = Rc::new(RefCell::new(None));
    let slides = Rc::new(RefCell::new(None));

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
        Rc::clone(&sheets),
        Rc::clone(&slides),
        settings_path.clone(),
    );
    bind_docs_actions(&ui, Rc::clone(&state), Rc::clone(&docs));
    bind_sheets_actions(&ui, Rc::clone(&state), Rc::clone(&sheets));
    bind_slides_actions(&ui, Rc::clone(&state), Rc::clone(&slides));
    bind_settings(&ui, Rc::clone(&state), settings_path.clone());
    bind_native_actions(
        &ui,
        Rc::clone(&state),
        Rc::clone(&docs),
        Rc::clone(&sheets),
        Rc::clone(&slides),
    );

    if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) {
        if is_docx_path(&path) {
            open_docs_path(&state, &docs, &ui.as_weak(), path);
        } else if is_xlsx_path(&path) {
            open_sheets_path(&state, &sheets, &ui.as_weak(), path);
        } else if is_pptx_path(&path) {
            open_slides_path(&state, &slides, &ui.as_weak(), path);
        } else {
            update_state(
                &state,
                &ui.as_weak(),
                settings_path.as_deref(),
                AppCommand::OpenFile(path),
            );
        }
    } else {
        let recovered = recovery_directory.as_deref().is_some_and(|root| {
            restore_latest_recovery(root, &state, &docs, &sheets, &slides, &ui)
        });
        if !recovered {
            sync_ui(&state.borrow(), &ui);
            sync_docs_ui(docs.borrow().as_ref(), &ui);
            sync_sheets_ui(sheets.borrow().as_ref(), &ui);
            sync_slides_ui(slides.borrow().as_ref(), &ui);
        }
    }

    ui.set_shell_init_text(format!("{:.1} ms", startup.elapsed().as_secs_f64() * 1000.0).into());
    ui.run()
}

fn restore_latest_recovery(
    root: &Path,
    state: &SharedState,
    docs: &SharedDocs,
    sheets: &SharedSheets,
    slides: &SharedSlides,
    ui: &AppWindow,
) -> bool {
    let Ok(Some(candidate)) = recovery_store::latest_candidate(root) else {
        return false;
    };

    let restored = match candidate.kind {
        RecoveryKind::Docs => DocsSession::open_recovery(candidate.snapshot, candidate.original)
            .map(|session| {
                *docs.borrow_mut() = Some(session);
                state.borrow_mut().apply(AppCommand::New(EditorKind::Docs));
            })
            .map_err(|error| error.to_string()),
        RecoveryKind::Sheets => {
            SheetsSession::open_recovery(candidate.snapshot, candidate.original)
                .map(|session| {
                    *sheets.borrow_mut() = Some(session);
                    state
                        .borrow_mut()
                        .apply(AppCommand::New(EditorKind::Sheets));
                })
                .map_err(|error| error.to_string())
        }
        RecoveryKind::Slides => {
            SlidesSession::open_recovery(candidate.snapshot, candidate.original)
                .map(|session| {
                    *slides.borrow_mut() = Some(session);
                    state
                        .borrow_mut()
                        .apply(AppCommand::New(EditorKind::Slides));
                })
                .map_err(|error| error.to_string())
        }
    };

    match restored {
        Ok(()) => {
            state
                .borrow_mut()
                .set_status("Recovered unsaved work from the previous session");
            sync_ui(&state.borrow(), ui);
            sync_docs_ui(docs.borrow().as_ref(), ui);
            sync_sheets_ui(sheets.borrow().as_ref(), ui);
            sync_slides_ui(slides.borrow().as_ref(), ui);
            true
        }
        Err(error) => {
            state
                .borrow_mut()
                .set_status(format!("Recovery failed: {error}"));
            sync_ui(&state.borrow(), ui);
            false
        }
    }
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
    sheets: SharedSheets,
    slides: SharedSlides,
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
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        let settings_path = settings_path.clone();
        ui.on_create_sheets(move || {
            *sheets.borrow_mut() = Some(SheetsSession::blank());
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_sheets_path_input("".into());
                ui.set_sheets_filter_input("".into());
            }
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::New(EditorKind::Sheets),
            );
            if let Some(ui) = ui_weak.upgrade() {
                sync_sheets_ui(sheets.borrow().as_ref(), &ui);
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_create_slides(move || {
            *slides.borrow_mut() = Some(SlidesSession::blank());
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_slides_path_input("".into());
            }
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::New(EditorKind::Slides),
            );
            if let Some(ui) = ui_weak.upgrade() {
                sync_slides_ui(slides.borrow().as_ref(), &ui);
            }
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

fn bind_sheets_actions(ui: &AppWindow, state: SharedState, sheets: SharedSheets) {
    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_open_path(move |path| {
            open_sheets_path(
                &state,
                &sheets,
                &ui_weak,
                PathBuf::from(path.trim().to_string()),
            );
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_save(move || {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.save()?;
                Ok(format!("Saved {}", session.title()))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_save_as(move |path| {
            let path = PathBuf::from(path.trim().to_string());
            apply_sheets_operation(&state, &sheets, &ui_weak, move |session| {
                session.save_as(path)?;
                Ok(format!("Saved {}", session.title()))
            });
            if let Some(ui) = ui_weak.upgrade()
                && let Some(session) = sheets.borrow().as_ref()
            {
                ui.set_sheets_path_input(session.path_text().into());
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_select_cell(move |row, column| {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.select_cell(row.max(0) as u32, column.max(0) as u32)?;
                Ok(format!("Cell {}", session.active_address()))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_select_address(move |address| {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.select_address(address.as_str())?;
                Ok(format!("Cell {}", session.active_address()))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_edit_cell(move |value| {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.set_active_input(value.as_str())?;
                Ok(format!("Edited {}", session.active_address()))
            });
        });
    }

    bind_sheets_format_action(
        ui,
        Rc::clone(&state),
        Rc::clone(&sheets),
        SheetsFormatAction::Bold,
    );
    bind_sheets_format_action(
        ui,
        Rc::clone(&state),
        Rc::clone(&sheets),
        SheetsFormatAction::Italic,
    );
    bind_sheets_format_action(
        ui,
        Rc::clone(&state),
        Rc::clone(&sheets),
        SheetsFormatAction::Fill,
    );

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_scroll(move |rows, columns| {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.scroll(rows, columns);
                Ok(format!("Cell {}", session.active_address()))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_sort(move |ascending| {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.sort_active_column(ascending)?;
                Ok(if ascending {
                    "Sorted ascending".into()
                } else {
                    "Sorted descending".into()
                })
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_filter(move |value| {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                let hidden = session.filter_active_column_equals(value.as_str())?;
                Ok(format!("Filtered {hidden} row(s)"))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_clear_filter(move || {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.clear_filter()?;
                Ok("Filter cleared".into())
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_freeze_row(move || {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.freeze_first_row()?;
                Ok("Freeze panes updated".into())
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_freeze_column(move || {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.freeze_first_column()?;
                Ok("Freeze panes updated".into())
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        let ui_weak = ui.as_weak();
        ui.on_sheets_add_sheet(move || {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                session.add_sheet()?;
                Ok(format!("Added {}", session.sheet_name()))
            });
        });
    }
}

#[derive(Clone, Copy)]
enum SheetsFormatAction {
    Bold,
    Italic,
    Fill,
}

fn bind_sheets_format_action(
    ui: &AppWindow,
    state: SharedState,
    sheets: SharedSheets,
    action: SheetsFormatAction,
) {
    let bind = move |ui_weak: slint::Weak<AppWindow>| {
        let state = Rc::clone(&state);
        let sheets = Rc::clone(&sheets);
        move || {
            apply_sheets_operation(&state, &sheets, &ui_weak, |session| {
                match action {
                    SheetsFormatAction::Bold => session.toggle_bold()?,
                    SheetsFormatAction::Italic => session.toggle_italic()?,
                    SheetsFormatAction::Fill => session.toggle_fill()?,
                }
                Ok("Formatting updated".into())
            });
        }
    };

    match action {
        SheetsFormatAction::Bold => ui.on_sheets_toggle_bold(bind(ui.as_weak())),
        SheetsFormatAction::Italic => ui.on_sheets_toggle_italic(bind(ui.as_weak())),
        SheetsFormatAction::Fill => ui.on_sheets_toggle_fill(bind(ui.as_weak())),
    }
}

fn bind_slides_actions(ui: &AppWindow, state: SharedState, slides: SharedSlides) {
    {
        let state = Rc::clone(&state);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_slides_open_path(move |path| {
            open_slides_path(
                &state,
                &slides,
                &ui_weak,
                PathBuf::from(path.trim().to_string()),
            );
        });
    }

    {
        let state = Rc::clone(&state);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_slides_save(move || {
            apply_slides_operation(&state, &slides, &ui_weak, |session| {
                session.save()?;
                Ok(format!("Saved {}", session.title()))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_slides_save_as(move |path| {
            let path = PathBuf::from(path.trim().to_string());
            apply_slides_operation(&state, &slides, &ui_weak, move |session| {
                session.save_as(path)?;
                Ok(format!("Saved {}", session.title()))
            });
            if let Some(ui) = ui_weak.upgrade()
                && let Some(session) = slides.borrow().as_ref()
            {
                ui.set_slides_path_input(session.path_text().into());
            }
        });
    }

    macro_rules! simple_slides_action {
        ($callback:ident, $method:ident, $status:expr) => {{
            let state = Rc::clone(&state);
            let slides = Rc::clone(&slides);
            let ui_weak = ui.as_weak();
            ui.$callback(move || {
                apply_slides_operation(&state, &slides, &ui_weak, |session| {
                    session.$method()?;
                    Ok(($status)(session))
                });
            });
        }};
    }

    simple_slides_action!(
        on_slides_previous,
        previous_slide,
        |session: &SlidesSession| { format!("Slide {}", session.current_slide_index() + 1) }
    );
    simple_slides_action!(on_slides_next, next_slide, |session: &SlidesSession| {
        format!("Slide {}", session.current_slide_index() + 1)
    });
    simple_slides_action!(
        on_slides_duplicate,
        duplicate_slide,
        |session: &SlidesSession| {
            format!("Duplicated slide {}", session.current_slide_index() + 1)
        }
    );
    simple_slides_action!(
        on_slides_delete_slide,
        delete_slide,
        |session: &SlidesSession| { format!("Slide {}", session.current_slide_index() + 1) }
    );
    simple_slides_action!(on_slides_add_text, add_text_box, |_| "Text box added"
        .into());
    simple_slides_action!(on_slides_add_shape, add_shape, |_| "Shape added".into());
    simple_slides_action!(on_slides_add_table, add_table, |_| "Table added".into());
    simple_slides_action!(on_slides_delete_element, delete_selected, |_| {
        "Element deleted".into()
    });
    simple_slides_action!(on_slides_bring_forward, move_selected_forward, |_| {
        "Element moved forward".into()
    });
    simple_slides_action!(on_slides_send_backward, move_selected_backward, |_| {
        "Element moved backward".into()
    });

    {
        let state = Rc::clone(&state);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_slides_add_slide(move || {
            apply_slides_operation(&state, &slides, &ui_weak, |session| {
                session.add_slide();
                Ok(format!("Added slide {}", session.current_slide_index() + 1))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_slides_select_element(move |index| {
            apply_slides_operation(&state, &slides, &ui_weak, |session| {
                session.select_element(index.max(0) as usize)?;
                Ok(format!("Selected element {}", index + 1))
            });
        });
    }

    {
        let state = Rc::clone(&state);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_slides_edit_text(move |text| {
            apply_slides_operation(&state, &slides, &ui_weak, |session| {
                session.edit_selected_text(text.as_str())?;
                Ok("Slide text updated".into())
            });
        });
    }
}

fn bind_native_actions(
    ui: &AppWindow,
    state: SharedState,
    docs: SharedDocs,
    sheets: SharedSheets,
    slides: SharedSlides,
) {
    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let sheets = Rc::clone(&sheets);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_native_open(move || match native_integration::choose_open_file() {
            Ok(Some(path)) => {
                open_office_path(&state, &docs, &sheets, &slides, &ui_weak, path);
            }
            Ok(None) => {}
            Err(error) => {
                state
                    .borrow_mut()
                    .set_status(format!("Native open dialog failed: {error}"));
                if let Some(ui) = ui_weak.upgrade() {
                    sync_ui(&state.borrow(), &ui);
                }
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let sheets = Rc::clone(&sheets);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_open_recent(move |path| {
            open_office_path(
                &state,
                &docs,
                &sheets,
                &slides,
                &ui_weak,
                PathBuf::from(path.as_str()),
            );
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let sheets = Rc::clone(&sheets);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_drop_office_data(move |data| match data.plain_text() {
            Ok(value) => {
                if let Some(path) = path_from_drop_text(value.as_str()) {
                    open_office_path(&state, &docs, &sheets, &slides, &ui_weak, path);
                } else {
                    state
                        .borrow_mut()
                        .set_status("Dropped data does not contain an Office file path");
                    if let Some(ui) = ui_weak.upgrade() {
                        sync_ui(&state.borrow(), &ui);
                    }
                }
            }
            Err(error) => {
                state
                    .borrow_mut()
                    .set_status(format!("Dropped data could not be read: {error}"));
                if let Some(ui) = ui_weak.upgrade() {
                    sync_ui(&state.borrow(), &ui);
                }
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let sheets = Rc::clone(&sheets);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_native_save_as(move || {
            let editor = state.borrow().active_editor();
            let (extension, suggested) = match editor {
                Some(EditorKind::Docs) => (
                    "docx",
                    docs.borrow()
                        .as_ref()
                        .map_or_else(|| "Untitled.docx".to_owned(), DocsSession::title),
                ),
                Some(EditorKind::Sheets) => (
                    "xlsx",
                    sheets
                        .borrow()
                        .as_ref()
                        .map_or_else(|| "Untitled.xlsx".to_owned(), SheetsSession::title),
                ),
                Some(EditorKind::Slides) => (
                    "pptx",
                    slides
                        .borrow()
                        .as_ref()
                        .map_or_else(|| "Untitled.pptx".to_owned(), SlidesSession::title),
                ),
                None => return,
            };

            match native_integration::choose_save_file(extension, &suggested) {
                Ok(Some(path)) => match editor {
                    Some(EditorKind::Docs) => {
                        apply_docs_operation(&state, &docs, &ui_weak, move |session| {
                            session.save_as(path)?;
                            Ok(format!("Saved {}", session.title()))
                        });
                    }
                    Some(EditorKind::Sheets) => {
                        apply_sheets_operation(&state, &sheets, &ui_weak, move |session| {
                            session.save_as(path)?;
                            Ok(format!("Saved {}", session.title()))
                        });
                    }
                    Some(EditorKind::Slides) => {
                        apply_slides_operation(&state, &slides, &ui_weak, move |session| {
                            session.save_as(path)?;
                            Ok(format!("Saved {}", session.title()))
                        });
                    }
                    None => {}
                },
                Ok(None) => {}
                Err(error) => {
                    state
                        .borrow_mut()
                        .set_status(format!("Native save dialog failed: {error}"));
                    if let Some(ui) = ui_weak.upgrade() {
                        sync_ui(&state.borrow(), &ui);
                    }
                }
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let sheets = Rc::clone(&sheets);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_native_print(move || {
            let current = current_file(&state.borrow(), &docs, &sheets, &slides);
            match current {
                Some((_, true)) => state
                    .borrow_mut()
                    .set_status("Save current changes before printing"),
                Some((path, false)) => match native_integration::print_file(&path) {
                    Ok(()) => state
                        .borrow_mut()
                        .set_status("Sent document to native print queue"),
                    Err(error) => state
                        .borrow_mut()
                        .set_status(format!("Native print failed: {error}")),
                },
                None => state
                    .borrow_mut()
                    .set_status("Save the document before printing"),
            }
            if let Some(ui) = ui_weak.upgrade() {
                sync_ui(&state.borrow(), &ui);
            }
        });
    }

    {
        let state = Rc::clone(&state);
        let docs = Rc::clone(&docs);
        let sheets = Rc::clone(&sheets);
        let slides = Rc::clone(&slides);
        let ui_weak = ui.as_weak();
        ui.on_native_copy_path(move || {
            let current = current_file(&state.borrow(), &docs, &sheets, &slides);
            match current {
                Some((path, _)) => match native_integration::copy_text(&path.to_string_lossy()) {
                    Ok(()) => state.borrow_mut().set_status("File path copied"),
                    Err(error) => state
                        .borrow_mut()
                        .set_status(format!("Clipboard command failed: {error}")),
                },
                None => state.borrow_mut().set_status("No saved file path to copy"),
            }
            if let Some(ui) = ui_weak.upgrade() {
                sync_ui(&state.borrow(), &ui);
            }
        });
    }
}

fn path_from_drop_text(value: &str) -> Option<PathBuf> {
    let first = value.lines().map(str::trim).find(|line| !line.is_empty())?;
    let decoded = first
        .strip_prefix("file://")
        .unwrap_or(first)
        .replace("%20", " ")
        .replace("%23", "#");
    let path = if cfg!(target_os = "windows")
        && decoded.starts_with('/')
        && decoded.as_bytes().get(2) == Some(&b':')
    {
        PathBuf::from(&decoded[1..])
    } else {
        PathBuf::from(decoded)
    };
    (is_docx_path(&path) || is_xlsx_path(&path) || is_pptx_path(&path)).then_some(path)
}

fn open_office_path(
    state: &SharedState,
    docs: &SharedDocs,
    sheets: &SharedSheets,
    slides: &SharedSlides,
    ui: &slint::Weak<AppWindow>,
    path: PathBuf,
) {
    if is_docx_path(&path) {
        open_docs_path(state, docs, ui, path);
    } else if is_xlsx_path(&path) {
        open_sheets_path(state, sheets, ui, path);
    } else if is_pptx_path(&path) {
        open_slides_path(state, slides, ui, path);
    } else {
        state
            .borrow_mut()
            .set_status("Unsupported Office file type");
        if let Some(ui) = ui.upgrade() {
            sync_ui(&state.borrow(), &ui);
        }
    }
}

fn current_file(
    state: &AppState,
    docs: &SharedDocs,
    sheets: &SharedSheets,
    slides: &SharedSlides,
) -> Option<(PathBuf, bool)> {
    match state.active_editor() {
        Some(EditorKind::Docs) => docs.borrow().as_ref().and_then(|session| {
            session
                .path()
                .map(|path| (path.to_path_buf(), session.is_dirty()))
        }),
        Some(EditorKind::Sheets) => sheets.borrow().as_ref().and_then(|session| {
            session
                .path()
                .map(|path| (path.to_path_buf(), session.is_dirty()))
        }),
        Some(EditorKind::Slides) => slides.borrow().as_ref().and_then(|session| {
            session
                .path()
                .map(|path| (path.to_path_buf(), session.is_dirty()))
        }),
        None => None,
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
            persist_recent_files(&state.borrow());
            if let Some(root) = platform::recovery_directory().as_deref() {
                let _ = recovery_store::clear(root, RecoveryKind::Docs);
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

fn open_slides_path(
    state: &SharedState,
    slides: &SharedSlides,
    ui: &slint::Weak<AppWindow>,
    path: PathBuf,
) {
    match SlidesSession::open(path.clone()) {
        Ok(session) => {
            *slides.borrow_mut() = Some(session);
            {
                let mut state = state.borrow_mut();
                state.apply(AppCommand::OpenFile(path));
                if let Some(session) = slides.borrow().as_ref() {
                    state.set_status(format!("Opened {}", session.title()));
                }
            }
            persist_recent_files(&state.borrow());
            if let Some(root) = platform::recovery_directory().as_deref() {
                let _ = recovery_store::clear(root, RecoveryKind::Slides);
            }
            if let Some(ui) = ui.upgrade() {
                if let Some(session) = slides.borrow().as_ref() {
                    ui.set_slides_path_input(session.path_text().into());
                }
                sync_ui(&state.borrow(), &ui);
                sync_slides_ui(slides.borrow().as_ref(), &ui);
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

fn open_sheets_path(
    state: &SharedState,
    sheets: &SharedSheets,
    ui: &slint::Weak<AppWindow>,
    path: PathBuf,
) {
    match SheetsSession::open(path.clone()) {
        Ok(session) => {
            *sheets.borrow_mut() = Some(session);
            {
                let mut state = state.borrow_mut();
                state.apply(AppCommand::OpenFile(path));
                if let Some(session) = sheets.borrow().as_ref() {
                    state.set_status(format!("Opened {}", session.title()));
                }
            }
            persist_recent_files(&state.borrow());
            if let Some(root) = platform::recovery_directory().as_deref() {
                let _ = recovery_store::clear(root, RecoveryKind::Sheets);
            }
            if let Some(ui) = ui.upgrade() {
                if let Some(session) = sheets.borrow().as_ref() {
                    ui.set_sheets_path_input(session.path_text().into());
                }
                sync_ui(&state.borrow(), &ui);
                sync_sheets_ui(sheets.borrow().as_ref(), &ui);
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

fn apply_slides_operation(
    state: &SharedState,
    slides: &SharedSlides,
    ui: &slint::Weak<AppWindow>,
    operation: impl FnOnce(&mut SlidesSession) -> Result<String, SlidesSessionError>,
) {
    let result = {
        let mut slides = slides.borrow_mut();
        match slides.as_mut() {
            Some(session) => operation(session),
            None => Ok("No Slides presentation is open".to_owned()),
        }
    };

    match result {
        Ok(status) => state.borrow_mut().set_status(status),
        Err(error) => state
            .borrow_mut()
            .set_status(format!("Slides command failed: {error}")),
    }

    if let Some(session) = slides.borrow_mut().as_mut() {
        maintain_slides_recovery(session);
    }

    if let Some(ui) = ui.upgrade() {
        sync_ui(&state.borrow(), &ui);
        sync_slides_ui(slides.borrow().as_ref(), &ui);
    }
}

fn apply_sheets_operation(
    state: &SharedState,
    sheets: &SharedSheets,
    ui: &slint::Weak<AppWindow>,
    operation: impl FnOnce(&mut SheetsSession) -> Result<String, SheetsSessionError>,
) {
    let result = {
        let mut sheets = sheets.borrow_mut();
        match sheets.as_mut() {
            Some(session) => operation(session),
            None => Ok("No Sheets workbook is open".to_owned()),
        }
    };

    match result {
        Ok(status) => state.borrow_mut().set_status(status),
        Err(error) => state
            .borrow_mut()
            .set_status(format!("Sheets command failed: {error}")),
    }

    if let Some(session) = sheets.borrow_mut().as_mut() {
        maintain_sheets_recovery(session);
    }

    if let Some(ui) = ui.upgrade() {
        sync_ui(&state.borrow(), &ui);
        sync_sheets_ui(sheets.borrow().as_ref(), &ui);
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

    if let Some(session) = docs.borrow_mut().as_mut() {
        maintain_docs_recovery(session);
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

fn persist_recent_files(state: &AppState) {
    let Some(path) = platform::recent_files_path() else {
        return;
    };
    if let Err(error) = recent_store::save(&path, state.recent_files()) {
        eprintln!("failed to persist recent files: {error}");
    }
}

fn maintain_docs_recovery(session: &mut DocsSession) {
    let original = session.path().map(Path::to_path_buf);
    maintain_recovery(
        RecoveryKind::Docs,
        original.as_deref(),
        session.is_dirty(),
        session.can_save(),
        |path| {
            session
                .save_recovery_copy(path)
                .map_err(|error| error.to_string())
        },
    );
}

fn maintain_sheets_recovery(session: &mut SheetsSession) {
    let original = session.path().map(Path::to_path_buf);
    maintain_recovery(
        RecoveryKind::Sheets,
        original.as_deref(),
        session.is_dirty(),
        session.can_save(),
        |path| {
            session
                .save_recovery_copy(path)
                .map_err(|error| error.to_string())
        },
    );
}

fn maintain_slides_recovery(session: &mut SlidesSession) {
    let original = session.path().map(Path::to_path_buf);
    maintain_recovery(
        RecoveryKind::Slides,
        original.as_deref(),
        session.is_dirty(),
        session.can_save(),
        |path| {
            session
                .save_recovery_copy(path)
                .map_err(|error| error.to_string())
        },
    );
}

fn maintain_recovery(
    kind: RecoveryKind,
    original: Option<&Path>,
    dirty: bool,
    can_save: bool,
    save_snapshot: impl FnOnce(&Path) -> Result<(), String>,
) {
    let Some(root) = platform::recovery_directory() else {
        return;
    };

    if !dirty {
        if let Err(error) = recovery_store::clear(&root, kind) {
            eprintln!("failed to clear recovery snapshot: {error}");
        }
        return;
    }
    if !can_save || !recovery_store::should_snapshot(&root, kind) {
        return;
    }

    let snapshot = recovery_store::snapshot_path(&root, kind);
    if let Err(error) = save_snapshot(&snapshot) {
        eprintln!("failed to write recovery snapshot: {error}");
        return;
    }
    if let Err(error) = recovery_store::record(&root, kind, original) {
        eprintln!("failed to write recovery metadata: {error}");
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
    let page = match state.active_editor() {
        Some(EditorKind::Docs) => 3,
        Some(EditorKind::Sheets) => 4,
        Some(EditorKind::Slides) => 5,
        _ => match state.page() {
            AppPage::Home => 0,
            AppPage::Diagnostics => 1,
            AppPage::Settings => 2,
        },
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
    let recent = state
        .recent_files()
        .iter()
        .take(3)
        .map(|path| RecentFileRow {
            label: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Office file")
                .into(),
            path: path.to_string_lossy().into_owned().into(),
        })
        .collect::<Vec<_>>();
    ui.set_recent_files(Rc::new(slint::VecModel::from(recent)).into());
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

fn sync_sheets_ui(session: Option<&SheetsSession>, ui: &AppWindow) {
    let is_chinese = ui.get_is_chinese();
    let Some(session) = session else {
        ui.set_sheets_title("Sheets".into());
        ui.set_sheets_current_path(if is_chinese {
            "未打开表格".into()
        } else {
            "No workbook open".into()
        });
        ui.set_sheets_dirty_meta("".into());
        ui.set_sheets_compatibility_text("".into());
        ui.set_sheets_sheet_name("".into());
        ui.set_sheets_address_input("A1".into());
        ui.set_sheets_cell_input("".into());
        ui.set_sheets_active_row(0);
        ui.set_sheets_active_column(0);
        ui.set_sheets_viewport_column(0);
        ui.set_sheets_bold(false);
        ui.set_sheets_italic(false);
        ui.set_sheets_can_save(false);
        ui.set_sheets_rows(Rc::new(slint::VecModel::from(Vec::<SheetRow>::new())).into());
        return;
    };

    ui.set_sheets_title(session.title().into());
    ui.set_sheets_current_path(if session.path_text().is_empty() {
        if is_chinese {
            "尚未保存".into()
        } else {
            "Not saved yet".into()
        }
    } else {
        session.path_text().into()
    });
    ui.set_sheets_dirty_meta(
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
    ui.set_sheets_compatibility_text(if session.can_save() {
        if is_chinese {
            "兼容性检查：可安全写入".into()
        } else {
            "Compatibility check: writable".into()
        }
    } else if is_chinese {
        format!(
            "已阻止保存 · {} 个暂不支持的结构",
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
    ui.set_sheets_sheet_name(
        format!(
            "{} · {}/{}",
            session.sheet_name(),
            session.active_sheet_index() + 1,
            session.sheet_count()
        )
        .into(),
    );
    ui.set_sheets_address_input(session.active_address().into());
    ui.set_sheets_cell_input(session.active_input().into());
    ui.set_sheets_active_row(session.active_cell().row as i32);
    ui.set_sheets_active_column(session.active_cell().column as i32);
    ui.set_sheets_viewport_column(session.viewport_column() as i32);
    ui.set_sheets_bold(session.active_bold());
    ui.set_sheets_italic(session.active_italic());
    ui.set_sheets_can_save(session.can_save());

    let header_values = session.viewport_header();
    ui.set_sheets_header(sheet_row_from_values(-1, "".into(), &header_values));

    let rows = session
        .viewport_rows()
        .into_iter()
        .map(sheet_row_from_data)
        .collect::<Vec<_>>();
    ui.set_sheets_rows(Rc::new(slint::VecModel::from(rows)).into());
}

fn sync_slides_ui(session: Option<&SlidesSession>, ui: &AppWindow) {
    let is_chinese = ui.get_is_chinese();
    let Some(session) = session else {
        ui.set_slides_title("Slides".into());
        ui.set_slides_current_path(if is_chinese {
            "未打开演示文稿".into()
        } else {
            "No presentation open".into()
        });
        ui.set_slides_dirty_meta("".into());
        ui.set_slides_compatibility_text("".into());
        ui.set_slides_slide_meta(if is_chinese {
            "第 0 页 / 共 0 页".into()
        } else {
            "Slide 0 of 0".into()
        });
        ui.set_slides_selected_text("".into());
        ui.set_slides_can_save(false);
        ui.set_slides_elements(
            Rc::new(slint::VecModel::from(Vec::<SlideElementRow>::new())).into(),
        );
        return;
    };

    ui.set_slides_title(session.title().into());
    ui.set_slides_current_path(if session.path_text().is_empty() {
        if is_chinese {
            "尚未保存".into()
        } else {
            "Not saved yet".into()
        }
    } else {
        session.path_text().into()
    });
    ui.set_slides_dirty_meta(
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
    ui.set_slides_compatibility_text(if session.can_save() {
        if is_chinese {
            "兼容性检查：可安全写入".into()
        } else {
            "Compatibility check: writable".into()
        }
    } else if is_chinese {
        format!(
            "已阻止保存 · {} 个暂不支持的结构",
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
    ui.set_slides_slide_meta(
        if is_chinese {
            format!(
                "第 {} 页 / 共 {} 页",
                session.current_slide_index() + 1,
                session.slide_count()
            )
        } else {
            format!(
                "Slide {} of {}",
                session.current_slide_index() + 1,
                session.slide_count()
            )
        }
        .into(),
    );
    ui.set_slides_selected_text(session.selected_text().into());
    ui.set_slides_can_save(session.can_save());
    let elements = session
        .element_summaries()
        .into_iter()
        .map(slide_element_row_from_summary)
        .collect::<Vec<_>>();
    ui.set_slides_elements(Rc::new(slint::VecModel::from(elements)).into());
}

fn slide_element_row_from_summary(summary: SlideElementSummary) -> SlideElementRow {
    SlideElementRow {
        index: summary.index as i32,
        kind: summary.kind.into(),
        text: summary.text.into(),
    }
}

fn sheet_row_from_data(row: SheetRowData) -> SheetRow {
    sheet_row_from_values(row.row as i32, row.row_label.into(), &row.cells)
}

fn sheet_row_from_values(
    row_index: i32,
    row_label: slint::SharedString,
    cells: &[String; 10],
) -> SheetRow {
    SheetRow {
        row_index,
        row_label,
        c0: cells[0].clone().into(),
        c1: cells[1].clone().into(),
        c2: cells[2].clone().into(),
        c3: cells[3].clone().into(),
        c4: cells[4].clone().into(),
        c5: cells[5].clone().into(),
        c6: cells[6].clone().into(),
        c7: cells[7].clone().into(),
        c8: cells[8].clone().into(),
        c9: cells[9].clone().into(),
    }
}

fn is_docx_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("docx"))
}

fn is_xlsx_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
}

fn is_pptx_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pptx"))
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
        "Recovered unsaved work from the previous session" => {
            "已恢复上次会话中未保存的内容".to_owned()
        }
        "Save current changes before printing" => "打印前请先保存当前更改".to_owned(),
        "Sent document to native print queue" => "已发送到系统打印队列".to_owned(),
        "Save the document before printing" => "打印前请先保存文件".to_owned(),
        "File path copied" => "已复制文件路径".to_owned(),
        "No saved file path to copy" => "当前没有可复制的已保存文件路径".to_owned(),
        "Unsupported Office file type" => "不支持的 Office 文件类型".to_owned(),
        "Dropped data does not contain an Office file path" => {
            "拖入的数据中没有可打开的 Office 文件路径".to_owned()
        }
        "Home" => "首页".to_owned(),
        "Diagnostics" => "诊断".to_owned(),
        "Settings" => "设置".to_owned(),
        "New Docs document" => "已新建文档".to_owned(),
        "New Sheets workbook" => "已新建表格".to_owned(),
        "New Slides presentation" => "已新建演示文稿".to_owned(),
        "No Slides presentation is open" => "当前未打开演示文稿".to_owned(),
        "Text box added" => "已添加文本框".to_owned(),
        "Shape added" => "已添加形状".to_owned(),
        "Table added" => "已添加表格".to_owned(),
        "Element deleted" => "已删除元素".to_owned(),
        "Element moved forward" => "元素已上移一层".to_owned(),
        "Element moved backward" => "元素已下移一层".to_owned(),
        "Slide text updated" => "幻灯片文字已更新".to_owned(),
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
        "No Sheets workbook is open" => "当前未打开表格".to_owned(),
        "Sorted ascending" => "已升序排序".to_owned(),
        "Sorted descending" => "已降序排序".to_owned(),
        "Filter cleared" => "已清除筛选".to_owned(),
        "Freeze panes updated" => "冻结窗格已更新".to_owned(),
        other if other.starts_with("Slide ") => {
            format!("第 {} 页", &other["Slide ".len()..])
        }
        other if other.starts_with("Added slide ") => {
            format!("已添加第 {} 页", &other["Added slide ".len()..])
        }
        other if other.starts_with("Duplicated slide ") => {
            format!("已复制第 {} 页", &other["Duplicated slide ".len()..])
        }
        other if other.starts_with("Selected element ") => {
            format!("已选择元素 {}", &other["Selected element ".len()..])
        }
        other if other.starts_with("Cell ") => {
            format!("单元格 {}", &other["Cell ".len()..])
        }
        other if other.starts_with("Edited ") => {
            format!("已编辑 {}", &other["Edited ".len()..])
        }
        other if other.starts_with("Filtered ") && other.ends_with(" row(s)") => {
            let count = other
                .trim_start_matches("Filtered ")
                .trim_end_matches(" row(s)");
            format!("已筛选，隐藏 {count} 行")
        }
        other if other.starts_with("Added ") => {
            format!("已添加 {}", &other["Added ".len()..])
        }
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
        other if other.starts_with("Sheets command failed: ") => {
            format!(
                "表格操作失败：{}",
                &other["Sheets command failed: ".len()..]
            )
        }
        other if other.starts_with("Slides command failed: ") => {
            format!(
                "演示文稿操作失败：{}",
                &other["Slides command failed: ".len()..]
            )
        }
        _ => status.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs, process,
        time::{SystemTime, UNIX_EPOCH},
    };

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
    fn dropped_paths_accept_office_paths_and_file_uris() {
        assert_eq!(
            path_from_drop_text("/tmp/report.docx"),
            Some(PathBuf::from("/tmp/report.docx"))
        );
        assert_eq!(
            path_from_drop_text("file:///tmp/Quarter%20Plan.xlsx"),
            Some(PathBuf::from("/tmp/Quarter Plan.xlsx"))
        );
        assert!(path_from_drop_text("/tmp/image.png").is_none());
    }

    #[test]
    fn phase8_same_process_editor_soak_round_trips() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow Unix epoch")
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("nexa-phase8-soak-{}-{nonce}", process::id()));
        fs::create_dir_all(&directory).unwrap();

        let docx_path = directory.join("soak.docx");
        let xlsx_path = directory.join("soak.xlsx");
        let pptx_path = directory.join("soak.pptx");

        for round in 0..16 {
            let mut docs = DocsSession::blank();
            docs.set_current_paragraph_text(&format!("第 {round} 轮 · Nexa Docs"))
                .unwrap();
            docs.insert_paragraph_after_current().unwrap();
            docs.set_current_paragraph_text("مرحبا · שלום · 🚀")
                .unwrap();
            docs.toggle_bold().unwrap();
            docs.save_as(docx_path.clone()).unwrap();
            let reopened = DocsSession::open(docx_path.clone()).unwrap();
            assert_eq!(reopened.paragraph_count(), 2);
            assert!(reopened.can_save());

            let mut sheets = SheetsSession::blank();
            sheets.select_address("A1").unwrap();
            sheets.set_active_input(&(round + 1).to_string()).unwrap();
            sheets.select_address("A2").unwrap();
            sheets.set_active_input("=A1*2").unwrap();
            sheets.toggle_bold().unwrap();
            sheets.save_as(xlsx_path.clone()).unwrap();
            let reopened = SheetsSession::open(xlsx_path.clone()).unwrap();
            assert!(reopened.can_save());
            assert_eq!(reopened.sheet_count(), 1);

            let mut slides = SlidesSession::blank();
            slides.add_text_box().unwrap();
            slides
                .edit_selected_text(&format!("Phase 8 round {round} · 中文"))
                .unwrap();
            slides.add_slide();
            slides.add_shape().unwrap();
            slides.edit_selected_text("Stable").unwrap();
            slides.save_as(pptx_path.clone()).unwrap();
            let reopened = SlidesSession::open(pptx_path.clone()).unwrap();
            assert!(reopened.can_save());
            assert_eq!(reopened.slide_count(), 2);
        }

        let _ = fs::remove_file(docx_path);
        let _ = fs::remove_file(xlsx_path);
        let _ = fs::remove_file(pptx_path);
        let _ = fs::remove_dir(directory);
    }

    #[test]
    fn english_status_remains_stable() {
        assert_eq!(
            localize_status("Opened quarterly.docx", false),
            "Opened quarterly.docx"
        );
    }
}
