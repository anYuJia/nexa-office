#![forbid(unsafe_code)]

use nexa_core::{AppCommand, AppState, EditorKind};
use std::{cell::RefCell, rc::Rc};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let state = Rc::new(RefCell::new(AppState::default()));

    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        ui.on_create_docs(move || {
            update_state(&state, &ui_weak, AppCommand::New(EditorKind::Docs));
        });
    }

    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        ui.on_create_sheets(move || {
            update_state(&state, &ui_weak, AppCommand::New(EditorKind::Sheets));
        });
    }

    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        ui.on_create_slides(move || {
            update_state(&state, &ui_weak, AppCommand::New(EditorKind::Slides));
        });
    }

    {
        let state = Rc::clone(&state);
        let ui_weak = ui.as_weak();
        ui.on_go_home(move || {
            update_state(&state, &ui_weak, AppCommand::GoHome);
        });
    }

    ui.set_status_text(state.borrow().status().into());
    ui.run()
}

fn update_state(
    state: &Rc<RefCell<AppState>>,
    ui: &slint::Weak<AppWindow>,
    command: AppCommand,
) {
    let status = {
        let mut state = state.borrow_mut();
        state.apply(command);
        state.status().to_owned()
    };

    if let Some(ui) = ui.upgrade() {
        ui.set_status_text(status.into());
    }
}
