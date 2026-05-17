use crate::api::models::CreateBlockRequest;
use crate::app_state::AppState;

/// Builds transactions, mines a block via MiningService, broadcasts.
/// Returns the new block height as string on success.
pub fn try_create_block(state: &AppState, req: &CreateBlockRequest) -> Result<String, String> {
    let mining_service = state
        .mining_service
        .as_ref()
        .ok_or_else(|| "MiningService not available".to_string())?;

    let txs: Vec<crate::storage::traits::Transaction> = req
        .transactions
        .iter()
        .map(|tx_req| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            crate::storage::traits::Transaction {
                id: uuid::Uuid::new_v4().to_string(),
                block_height: 0,
                timestamp: now,
                input_did: tx_req.from.clone(),
                output_recipient: tx_req.to.clone(),
                amount: tx_req.amount,
                state: "pending".to_string(),
            }
        })
        .collect();

    let miner = req
        .transactions
        .first()
        .map(|t| t.from.as_str())
        .unwrap_or("system");

    let height = mining_service.mine_block(miner, txs)?;
    Ok(format!("block-{height}"))
}
