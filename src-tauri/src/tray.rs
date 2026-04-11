#[derive(Debug, Clone)]
pub struct TrayModel {
    pub items: Vec<String>,
}

pub fn build_tray_model(current_mode: &str) -> TrayModel {
    TrayModel {
        items: vec![
            format!("current-mode:{current_mode}"),
            "mode:account_only".into(),
            "mode:relay_only".into(),
            "mode:hybrid".into(),
            "action:open".into(),
            "action:restart-gateway".into(),
            "action:quit".into(),
        ],
    }
}
