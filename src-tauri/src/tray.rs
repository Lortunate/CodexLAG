use std::borrow::Cow;

use crate::{
    gateway::server::LOOPBACK_GATEWAY_LISTEN_ADDRESS,
    routing::policy::RoutingMode,
    state::{AppState, RuntimeState},
    tray_summary::{build_tray_summary_for_runtime, TraySummaryModel},
};
use tauri::{
    menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager, Runtime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayModel {
    pub items: Vec<TrayItem>,
}

impl TrayModel {
    pub fn current_mode(&self) -> Option<RoutingMode> {
        self.items.iter().find_map(|item| match &item.label {
            TrayItemLabel::Summary {
                mode: Some(mode), ..
            } => Some(*mode),
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
    GatewayStatus,
    ListenAddress,
    AvailableEndpoints,
    LastBalanceRefresh,
    Mode(RoutingMode),
    Open,
    RestartGateway,
    Quit,
}

impl TrayItemId {
    pub fn menu_id(self) -> Cow<'static, str> {
        match self {
            Self::CurrentMode => Cow::Borrowed("status:current_mode"),
            Self::GatewayStatus => Cow::Borrowed("status:gateway_status"),
            Self::ListenAddress => Cow::Borrowed("status:listen_address"),
            Self::AvailableEndpoints => Cow::Borrowed("status:available_endpoints"),
            Self::LastBalanceRefresh => Cow::Borrowed("status:last_balance_refresh"),
            Self::Mode(mode) => format!("mode:{}", mode.as_str()).into(),
            Self::Open => Cow::Borrowed("action:open"),
            Self::RestartGateway => Cow::Borrowed("action:restart_gateway"),
            Self::Quit => Cow::Borrowed("action:quit"),
        }
    }

    fn from_menu_id(value: &str) -> Option<Self> {
        match value {
            "status:current_mode" => Some(Self::CurrentMode),
            "status:gateway_status" => Some(Self::GatewayStatus),
            "status:listen_address" => Some(Self::ListenAddress),
            "status:available_endpoints" => Some(Self::AvailableEndpoints),
            "status:last_balance_refresh" => Some(Self::LastBalanceRefresh),
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
    Summary {
        mode: Option<RoutingMode>,
        text: String,
    },
    Mode(RoutingMode),
    Open,
    RestartGateway,
    Quit,
}

impl TrayItemLabel {
    pub fn text(&self) -> Cow<'static, str> {
        match self {
            Self::Summary { text, .. } => Cow::Owned(text.clone()),
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

fn summary_item(id: TrayItemId, text: String, mode: Option<RoutingMode>) -> TrayItem {
    TrayItem {
        id,
        label: TrayItemLabel::Summary { mode, text },
        kind: TrayItemKind::Status,
    }
}

pub fn build_tray_model(summary: TraySummaryModel) -> TrayModel {
    let mut items = vec![
        summary_item(
            TrayItemId::CurrentMode,
            summary.current_mode_label,
            Some(summary.current_mode),
        ),
        summary_item(
            TrayItemId::GatewayStatus,
            summary.gateway_status_label,
            None,
        ),
        summary_item(
            TrayItemId::ListenAddress,
            summary.listen_address_label,
            None,
        ),
        summary_item(
            TrayItemId::AvailableEndpoints,
            summary.available_endpoints_label,
            None,
        ),
        summary_item(
            TrayItemId::LastBalanceRefresh,
            summary.last_balance_refresh_label,
            None,
        ),
    ];

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
    let current_mode = state.current_mode().unwrap_or(RoutingMode::Hybrid);
    build_tray_model(TraySummaryModel {
        current_mode,
        current_mode_label: format!(
            "Default key state | Current mode: {}",
            current_mode.as_str()
        ),
        gateway_status_label: "Gateway status | ready".to_string(),
        listen_address_label: format!("Listen address | {LOOPBACK_GATEWAY_LISTEN_ADDRESS}"),
        available_endpoints_label: "Available endpoints | official: 0, relay: 0".to_string(),
        last_balance_refresh_label: "Last balance refresh | none".to_string(),
    })
}

pub fn build_tray_model_for_runtime(runtime: &RuntimeState) -> TrayModel {
    build_tray_model(build_tray_summary_for_runtime(runtime))
}

pub fn apply_tray_action_for_runtime(
    runtime: &RuntimeState,
    item_id: TrayItemId,
) -> crate::error::Result<TrayModel> {
    match item_id {
        TrayItemId::Mode(mode) => runtime.set_current_mode(mode)?,
        TrayItemId::RestartGateway => runtime.restart_gateway()?,
        TrayItemId::CurrentMode
        | TrayItemId::GatewayStatus
        | TrayItemId::ListenAddress
        | TrayItemId::AvailableEndpoints
        | TrayItemId::LastBalanceRefresh
        | TrayItemId::Open
        | TrayItemId::Quit => {}
    }

    Ok(build_tray_model_for_runtime(runtime))
}

fn label_text<'a>(model: &'a TrayModel, id: TrayItemId, fallback: &'static str) -> Cow<'a, str> {
    model
        .items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.text())
        .unwrap_or_else(|| Cow::Borrowed(fallback))
}

pub fn install_runtime_tray<R: Runtime>(app: &App<R>, model: &TrayModel) -> tauri::Result<()> {
    let current_mode_item = MenuItem::with_id(
        app,
        TrayItemId::CurrentMode.menu_id().as_ref(),
        label_text(
            model,
            TrayItemId::CurrentMode,
            "Default key state | Current mode: hybrid",
        )
        .as_ref(),
        false,
        None::<&str>,
    )?;
    let gateway_status_item = MenuItem::with_id(
        app,
        TrayItemId::GatewayStatus.menu_id().as_ref(),
        label_text(model, TrayItemId::GatewayStatus, "Gateway status | ready").as_ref(),
        false,
        None::<&str>,
    )?;
    let listen_address_item = MenuItem::with_id(
        app,
        TrayItemId::ListenAddress.menu_id().as_ref(),
        label_text(
            model,
            TrayItemId::ListenAddress,
            "Listen address | http://127.0.0.1:8787",
        )
        .as_ref(),
        false,
        None::<&str>,
    )?;
    let available_endpoints_item = MenuItem::with_id(
        app,
        TrayItemId::AvailableEndpoints.menu_id().as_ref(),
        label_text(
            model,
            TrayItemId::AvailableEndpoints,
            "Available endpoints | official: 0, relay: 0",
        )
        .as_ref(),
        false,
        None::<&str>,
    )?;
    let last_balance_refresh_item = MenuItem::with_id(
        app,
        TrayItemId::LastBalanceRefresh.menu_id().as_ref(),
        label_text(
            model,
            TrayItemId::LastBalanceRefresh,
            "Last balance refresh | none",
        )
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
        .item(&gateway_status_item)
        .item(&listen_address_item)
        .item(&available_endpoints_item)
        .item(&last_balance_refresh_item)
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
                    &gateway_status_item,
                    &listen_address_item,
                    &available_endpoints_item,
                    &last_balance_refresh_item,
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
    gateway_status_item: &MenuItem<R>,
    listen_address_item: &MenuItem<R>,
    available_endpoints_item: &MenuItem<R>,
    last_balance_refresh_item: &MenuItem<R>,
    account_only_item: &CheckMenuItem<R>,
    relay_only_item: &CheckMenuItem<R>,
    hybrid_item: &CheckMenuItem<R>,
) {
    match item_id {
        TrayItemId::Open => show_main_window(app),
        TrayItemId::Quit => app.exit(0),
        TrayItemId::Mode(_) | TrayItemId::RestartGateway => {
            if let Some(runtime) = app.try_state::<RuntimeState>() {
                if let Ok(model) = apply_tray_action_for_runtime(&runtime, item_id) {
                    apply_tray_model_to_menu(
                        &model,
                        current_mode_item,
                        gateway_status_item,
                        listen_address_item,
                        available_endpoints_item,
                        last_balance_refresh_item,
                        account_only_item,
                        relay_only_item,
                        hybrid_item,
                    );
                    if let Ok(summary) =
                        crate::commands::keys::default_key_summary_from_runtime(&runtime)
                    {
                        let _ =
                            crate::commands::keys::emit_default_key_summary_changed(app, &summary);
                    }
                }
            }
        }
        TrayItemId::CurrentMode
        | TrayItemId::GatewayStatus
        | TrayItemId::ListenAddress
        | TrayItemId::AvailableEndpoints
        | TrayItemId::LastBalanceRefresh => {}
    }
}

fn apply_tray_model_to_menu<R: Runtime>(
    model: &TrayModel,
    current_mode_item: &MenuItem<R>,
    gateway_status_item: &MenuItem<R>,
    listen_address_item: &MenuItem<R>,
    available_endpoints_item: &MenuItem<R>,
    last_balance_refresh_item: &MenuItem<R>,
    account_only_item: &CheckMenuItem<R>,
    relay_only_item: &CheckMenuItem<R>,
    hybrid_item: &CheckMenuItem<R>,
) {
    let _ = current_mode_item.set_text(
        label_text(
            model,
            TrayItemId::CurrentMode,
            "Default key state | Current mode: hybrid",
        )
        .as_ref(),
    );
    let _ = gateway_status_item
        .set_text(label_text(model, TrayItemId::GatewayStatus, "Gateway status | ready").as_ref());
    let _ = listen_address_item.set_text(
        label_text(
            model,
            TrayItemId::ListenAddress,
            "Listen address | http://127.0.0.1:8787",
        )
        .as_ref(),
    );
    let _ = available_endpoints_item.set_text(
        label_text(
            model,
            TrayItemId::AvailableEndpoints,
            "Available endpoints | official: 0, relay: 0",
        )
        .as_ref(),
    );
    let _ = last_balance_refresh_item.set_text(
        label_text(
            model,
            TrayItemId::LastBalanceRefresh,
            "Last balance refresh | none",
        )
        .as_ref(),
    );

    let current_mode = model.current_mode();
    let _ = account_only_item.set_checked(current_mode == Some(RoutingMode::AccountOnly));
    let _ = relay_only_item.set_checked(current_mode == Some(RoutingMode::RelayOnly));
    let _ = hybrid_item.set_checked(current_mode == Some(RoutingMode::Hybrid));
}

fn show_main_window<R: Runtime, M: Manager<R>>(manager: &M) {
    if let Some(window) = manager.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
