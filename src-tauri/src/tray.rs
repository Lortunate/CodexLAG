use std::borrow::Cow;

use crate::routing::policy::RoutingMode;
use crate::state::AppState;
use tauri::{
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager, Runtime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayModel {
    pub items: Vec<TrayItem>,
}

impl TrayModel {
    pub fn current_mode(&self) -> Option<RoutingMode> {
        self.items.iter().find_map(|item| match item.label {
            TrayItemLabel::CurrentMode(mode, _) => Some(mode),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayItem {
    pub id: TrayItemId,
    pub label: TrayItemLabel,
    pub kind: TrayItemKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayItemId {
    CurrentMode,
    Mode(RoutingMode),
    Open,
    RestartGateway,
    Quit,
}

impl TrayItemId {
    pub fn menu_id(self) -> Cow<'static, str> {
        match self {
            Self::CurrentMode => Cow::Borrowed("status:current_mode"),
            Self::Mode(mode) => format!("mode:{}", mode.as_str()).into(),
            Self::Open => Cow::Borrowed("action:open"),
            Self::RestartGateway => Cow::Borrowed("action:restart_gateway"),
            Self::Quit => Cow::Borrowed("action:quit"),
        }
    }

    fn from_menu_id(value: &str) -> Option<Self> {
        match value {
            "status:current_mode" => Some(Self::CurrentMode),
            "action:open" => Some(Self::Open),
            "action:restart_gateway" => Some(Self::RestartGateway),
            "action:quit" => Some(Self::Quit),
            _ => value
                .strip_prefix("mode:")
                .and_then(RoutingMode::parse)
                .map(Self::Mode),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayItemLabel {
    CurrentMode(RoutingMode, Option<String>),
    Mode(RoutingMode),
    Open,
    RestartGateway,
    Quit,
}

impl TrayItemLabel {
    pub fn text(&self) -> Cow<'static, str> {
        match self {
            Self::CurrentMode(mode, Some(reason)) => format!(
                "Default key state | Current mode: {} ({reason})",
                mode.as_str()
            )
            .into(),
            Self::CurrentMode(mode, None) => {
                format!("Default key state | Current mode: {}", mode.as_str()).into()
            }
            Self::Mode(mode) => match mode {
                RoutingMode::AccountOnly => Cow::Borrowed("Account only"),
                RoutingMode::RelayOnly => Cow::Borrowed("Relay only"),
                RoutingMode::Hybrid => Cow::Borrowed("Hybrid"),
            },
            Self::Open => Cow::Borrowed("Open"),
            Self::RestartGateway => Cow::Borrowed("Restart gateway"),
            Self::Quit => Cow::Borrowed("Quit"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayItemKind {
    Status,
    Mode,
    Action,
}

pub fn build_tray_model(current_mode: RoutingMode, unavailable_reason: Option<String>) -> TrayModel {
    let mut items = vec![TrayItem {
        id: TrayItemId::CurrentMode,
        label: TrayItemLabel::CurrentMode(current_mode, unavailable_reason),
        kind: TrayItemKind::Status,
    }];

    items.extend(RoutingMode::ALL.into_iter().map(|mode| TrayItem {
        id: TrayItemId::Mode(mode),
        label: TrayItemLabel::Mode(mode),
        kind: TrayItemKind::Mode,
    }));

    items.extend([
        TrayItem {
            id: TrayItemId::Open,
            label: TrayItemLabel::Open,
            kind: TrayItemKind::Action,
        },
        TrayItem {
            id: TrayItemId::RestartGateway,
            label: TrayItemLabel::RestartGateway,
            kind: TrayItemKind::Action,
        },
        TrayItem {
            id: TrayItemId::Quit,
            label: TrayItemLabel::Quit,
            kind: TrayItemKind::Action,
        },
    ]);

    TrayModel { items }
}

pub fn build_tray_model_for_state(state: &AppState) -> TrayModel {
    build_tray_model(state.current_mode().unwrap_or(RoutingMode::Hybrid), None)
}

pub fn install_runtime_tray<R: Runtime>(app: &App<R>, model: &TrayModel) -> tauri::Result<()> {
    let current_mode_item = MenuItem::with_id(
        app,
        TrayItemId::CurrentMode.menu_id().as_ref(),
        model
            .items
            .iter()
                .find(|item| item.id == TrayItemId::CurrentMode)
                .map(|item| item.label.text())
                .unwrap_or_else(|| Cow::Borrowed("Default key state | Current mode: hybrid"))
            .as_ref(),
        false,
        None::<&str>,
    )?;

    let account_only_item = CheckMenuItemBuilder::with_id(
        TrayItemId::Mode(RoutingMode::AccountOnly)
            .menu_id()
            .as_ref(),
        TrayItemLabel::Mode(RoutingMode::AccountOnly)
            .text()
            .as_ref(),
    )
    .checked(model.current_mode() == Some(RoutingMode::AccountOnly))
    .build(app)?;
    let relay_only_item = CheckMenuItemBuilder::with_id(
        TrayItemId::Mode(RoutingMode::RelayOnly).menu_id().as_ref(),
        TrayItemLabel::Mode(RoutingMode::RelayOnly).text().as_ref(),
    )
    .checked(model.current_mode() == Some(RoutingMode::RelayOnly))
    .build(app)?;
    let hybrid_item = CheckMenuItemBuilder::with_id(
        TrayItemId::Mode(RoutingMode::Hybrid).menu_id().as_ref(),
        TrayItemLabel::Mode(RoutingMode::Hybrid).text().as_ref(),
    )
    .checked(model.current_mode() == Some(RoutingMode::Hybrid))
    .build(app)?;
    let open_item = MenuItem::with_id(
        app,
        TrayItemId::Open.menu_id().as_ref(),
        TrayItemLabel::Open.text().as_ref(),
        true,
        None::<&str>,
    )?;
    let restart_item = MenuItem::with_id(
        app,
        TrayItemId::RestartGateway.menu_id().as_ref(),
        TrayItemLabel::RestartGateway.text().as_ref(),
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(
        app,
        TrayItemId::Quit.menu_id().as_ref(),
        TrayItemLabel::Quit.text().as_ref(),
        true,
        None::<&str>,
    )?;

    let menu = MenuBuilder::new(app)
        .item(&current_mode_item)
        .separator()
        .item(&account_only_item)
        .item(&relay_only_item)
        .item(&hybrid_item)
        .separator()
        .item(&open_item)
        .item(&restart_item)
        .item(&quit_item)
        .build()?;

    TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            if let Some(item_id) = TrayItemId::from_menu_id(event.id().as_ref()) {
                handle_menu_event(
                    app,
                    item_id,
                    &current_mode_item,
                    &account_only_item,
                    &relay_only_item,
                    &hybrid_item,
                );
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                show_main_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

fn handle_menu_event<R: Runtime>(
    app: &tauri::AppHandle<R>,
    item_id: TrayItemId,
    current_mode_item: &MenuItem<R>,
    account_only_item: &tauri::menu::CheckMenuItem<R>,
    relay_only_item: &tauri::menu::CheckMenuItem<R>,
    hybrid_item: &tauri::menu::CheckMenuItem<R>,
) {
    match item_id {
        TrayItemId::Open => show_main_window(app),
        TrayItemId::Quit => app.exit(0),
        TrayItemId::Mode(mode) => {
            if let Some(runtime) = app.try_state::<crate::state::RuntimeState>() {
                let _ = runtime.set_current_mode(mode);
                if let Ok(summary) =
                    crate::commands::keys::default_key_summary_from_runtime(&runtime)
                {
                    let _ = crate::commands::keys::emit_default_key_summary_changed(app, &summary);
                    let reason = summary.unavailable_reason.clone();
                    let _ = current_mode_item
                        .set_text(TrayItemLabel::CurrentMode(mode, reason).text().as_ref());
                } else {
                    let _ = current_mode_item.set_text(TrayItemLabel::CurrentMode(mode, None).text().as_ref());
                }
                let _ = account_only_item.set_checked(mode == RoutingMode::AccountOnly);
                let _ = relay_only_item.set_checked(mode == RoutingMode::RelayOnly);
                let _ = hybrid_item.set_checked(mode == RoutingMode::Hybrid);
            }
        }
        TrayItemId::CurrentMode | TrayItemId::RestartGateway => {}
    }
}

fn show_main_window<R: Runtime, M: Manager<R>>(manager: &M) {
    if let Some(window) = manager.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
