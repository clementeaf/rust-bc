use crate::api::models::CreateBlockRequest;
use crate::app_state::AppState;
use crate::models::Transaction;

/// Builds transactions, mines a block, persists, broadcasts, updates cache.
/// Returns the new block hash on success.
pub fn try_create_block(state: &AppState, req: &CreateBlockRequest) -> Result<String, String> {
    let mut blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let mut wallet_manager = state
        .wallet_manager
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let transactions: Result<Vec<Transaction>, String> = req
        .transactions
        .iter()
        .map(|tx_req| {
            let fee = tx_req.fee.unwrap_or(0);
            let mut tx = Transaction::new_with_fee(
                tx_req.from.clone(),
                tx_req.to.clone(),
                tx_req.amount,
                fee,
                tx_req.data.clone(),
            );

            if tx_req.from != "0" {
                let wallet = wallet_manager
                    .get_wallet_for_signing(&tx_req.from)
                    .ok_or_else(|| "Wallet no encontrado para firmar".to_string())?;
                wallet.sign_transaction(&mut tx);
            }

            Ok(tx)
        })
        .collect();

    match transactions {
        Ok(txs) => {
            let mut mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
            for tx in &txs {
                if tx.from != "0" {
                    mempool.remove_transaction(&tx.id);
                }
            }
            drop(mempool);

            match blockchain.add_block(txs.clone(), &wallet_manager) {
                Ok(hash) => {
                    for tx in &txs {
                        if tx.from == "0" {
                            let _ = wallet_manager.process_coinbase_transaction(tx);
                        } else {
                            let _ = wallet_manager.process_transaction(tx);
                        }
                    }

                    let latest = blockchain.get_latest_block();
                    let latest_index = latest.index;
                    let latest_block_clone = latest.clone();

                    if let Some(ref storage) = state.block_storage {
                        if let Err(e) = storage.save_block(&latest_block_clone) {
                            eprintln!("⚠️  Error al guardar bloque en archivos: {}", e);
                        }
                    }

                    if let Some(node) = &state.node {
                        let node_clone = node.clone();
                        tokio::spawn(async move {
                            node_clone.broadcast_block(&latest_block_clone).await;
                        });
                    }

                    drop(blockchain);
                    state.balance_cache.invalidate(latest_index);

                    Ok(hash)
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}
