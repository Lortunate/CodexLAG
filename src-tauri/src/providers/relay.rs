use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedBalance {
    pub total: String,
    pub used: String,
}

#[derive(Debug, Deserialize)]
struct NewApiPayload {
    data: NewApiBalanceData,
}

#[derive(Debug, Deserialize)]
struct NewApiBalanceData {
    total_balance: String,
    used_balance: String,
}

pub fn normalize_newapi_balance_response(
    body: &str,
) -> Result<NormalizedBalance, serde_json::Error> {
    let payload: NewApiPayload = serde_json::from_str(body)?;
    Ok(NormalizedBalance {
        total: payload.data.total_balance,
        used: payload.data.used_balance,
    })
}
