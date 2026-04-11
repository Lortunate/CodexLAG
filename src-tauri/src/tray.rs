use std::borrow::Cow;

use crate::routing::policy::RoutingMode;
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayModel {
    pub items: Vec<TrayItem>,
}

impl TrayModel {
    pub fn current_mode(&self) -> Option<RoutingMode> {
        self.items.iter().find_map(|item| match item.label {
            TrayItemLabel::CurrentMode(mode) => Some(mode),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayItemLabel {
    CurrentMode(RoutingMode),
    Mode(RoutingMode),
    Open,
    RestartGateway,
    Quit,
}

impl TrayItemLabel {
    pub fn text(self) -> Cow<'static, str> {
        match self {
            Self::CurrentMode(mode) => format!("Current mode: {}", mode.as_str()).into(),
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

pub fn build_tray_model(current_mode: RoutingMode) -> TrayModel {
    let mut items = vec![TrayItem {
        id: TrayItemId::CurrentMode,
        label: TrayItemLabel::CurrentMode(current_mode),
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
    build_tray_model(state.current_mode().unwrap_or(RoutingMode::Hybrid))
}
