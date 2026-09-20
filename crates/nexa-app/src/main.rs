#![deny(unsafe_code)]

mod platform;
mod settings_store;

use nexa_core::{AppCommand, AppPage, AppSettings, AppState, EditorKind};
use platform::PlatformInfo;
use std::{cell::RefCell, path::PathBuf, rc::Rc};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let settings_path = platform::settings_path();
    let settings = load_settings(settings_path.as_deref());
    let state = Rc::new(RefCell::new(AppState::with_settings(settings)));

    configure_static_diagnostics(&ui);

    bind_navigation(&ui, Rc::clone(&state), settings_path.clone());
    bind_editor_actions(&ui, Rc::clone(&state), settings_path.clone());
    bind_settings(&ui, Rc::clone(&state), settings_path.clone());

    if let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) {
        update_state(
            &state,
            &ui.as_weak(),
            settings_path.as_deref(),
            AppCommand::OpenFile(path),
        );
    } else {
        sync_ui(&state.borrow(), &ui);
    }

    ui.run()
}

fn load_settings(path: Option<&std::path::Path>) -> AppSettings {
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

fn bind_navigation(ui: &AppWindow, state: Rc<RefCell<AppState>>, settings_path: Option<PathBuf>) {
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
    state: Rc<RefCell<AppState>>,
    settings_path: Option<PathBuf>,
) {
    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        let settings_path = settings_path.clone();
        ui.on_create_docs(move || {
            update_state(
                &state,
                &ui_weak,
                settings_path.as_deref(),
                AppCommand::New(EditorKind::Docs),
            );
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

fn bind_settings(ui: &AppWindow, state: Rc<RefCell<AppState>>, settings_path: Option<PathBuf>) {
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
}

fn update_state(
    state: &Rc<RefCell<AppState>>,
    ui: &slint::Weak<AppWindow>,
    settings_path: Option<&std::path::Path>,
    command: AppCommand,
) {
    let persist_settings = matches!(
        &command,
        AppCommand::SetShowStatusBar(_) | AppCommand::SetCompactNavigation(_)
    );

    {
        let mut state = state.borrow_mut();
        state.apply(command);

        if persist_settings {
            persist_settings_if_available(settings_path, state.settings());
        }
    }

    if let Some(ui) = ui.upgrade() {
        sync_ui(&state.borrow(), &ui);
    }
}

fn persist_settings_if_available(settings_path: Option<&std::path::Path>, settings: &AppSettings) {
    let Some(path) = settings_path else {
        return;
    };

    let _ = settings_store::save(path, settings);
}

fn sync_ui(state: &AppState, ui: &AppWindow) {
    let page = match state.page() {
        AppPage::Home => 0,
        AppPage::Diagnostics => 1,
        AppPage::Settings => 2,
    };

    ui.set_page(page);
    ui.set_status_text(state.status().into());
    ui.set_show_status_bar(state.settings().show_status_bar());
    ui.set_compact_navigation(state.settings().compact_navigation());
}
