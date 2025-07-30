use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use solana_sdk::{bs58, transaction::Transaction};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::{
    executor::{Executor, STATE},
    validator::TransactionValidator,
};

// App state
#[derive(Clone)]
pub struct AppState {
    executor: Arc<Executor>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitTransactionRequest {
    pub transaction: String, // Base58 encoded txn
}

#[derive(Debug, Serialize)]
pub struct SubmitTransactionResponse {
    pub transaction_id: String,
    pub status: String,
    pub message: String,
}

pub async fn start() {
    // Step1. create app state
    let state: AppState = AppState {
        executor: Arc::new(Executor::new()),
    };

    // Step2. create router
    let app = Router::new()
        .route("/submit_transaction", post(submit_transaction))
        .route("/get_balance", post(get_balance))
        .layer(CorsLayer::permissive())
        .with_state(state);

    info!("Starting node rpc server on 0.0.0.0:8898");

    // Step3. start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8898").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// handle txn
async fn submit_transaction(
    State(state): State<AppState>,
    Json(payload): Json<SubmitTransactionRequest>,
) -> Result<Json<SubmitTransactionResponse>, StatusCode> {
    info!("Received transaction submission request");

    // decode solana txn
    let transaction_bytes = match bs58::decode(&payload.transaction).into_vec() {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to decode transaction: {}", e);
            return Ok(Json(SubmitTransactionResponse {
                transaction_id: String::new(),
                status: "error".to_string(),
                message: format!("Invalid transaction encoding: {}", e),
            }));
        }
    };

    let transaction: Transaction = match bincode::deserialize(&transaction_bytes) {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to deserialize transaction: {}", e);
            return Ok(Json(SubmitTransactionResponse {
                transaction_id: String::new(),
                status: "error".to_string(),
                message: format!("Invalid transaction format: {}", e),
            }));
        }
    };

    let signature = transaction.signatures[0].to_string();
    info!("Processing transaction with signature: {}", signature);

    // verify txn
    match TransactionValidator::validate_transaction(&transaction).await {
        Ok(_) => {
            info!("Transaction validation passed");
        }
        Err(e) => {
            warn!("Transaction validation failed: {}", e);
            return Ok(Json(SubmitTransactionResponse {
                transaction_id: signature.clone(),
                status: "rejected".to_string(),
                message: format!("Transaction validation failed: {}", e),
            }));
        }
    }

    // add txn to mempool
    match state.executor.add_tnx(transaction).await {
        Ok(_) => {
            info!("Transaction processed successfully: {}", signature);
            Ok(Json(SubmitTransactionResponse {
                transaction_id: signature,
                status: "success".to_string(),
                message: "Transaction processed successfully".to_string(),
            }))
        }
        Err(e) => {
            error!("Transaction processing failed: {}", e);
            Ok(Json(SubmitTransactionResponse {
                transaction_id: signature,
                status: "failed".to_string(),
                message: format!("Transaction processing failed: {}", e),
            }))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetBalanceRequest {
    pub pubkey: String,
}

#[derive(Debug, Serialize)]
pub struct GetBalanceResponse {
    pub balance: u128,
    pub pubkey: String,
}

// get user balance
async fn get_balance(
    Json(payload): Json<GetBalanceRequest>,
) -> Result<Json<GetBalanceResponse>, StatusCode> {
    let state_db = STATE.read().await;
    let balance = state_db.state.get_balance(&payload.pubkey);

    Ok(Json(GetBalanceResponse {
        balance,
        pubkey: payload.pubkey,
    }))
}
