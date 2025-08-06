use axum::{http::StatusCode, response::Json, routing::post, Router};
use base64::{self, engine::general_purpose, Engine};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use share::utils::read_env_var;
use solana_sdk::{bs58, transaction::Transaction};
use solana_transaction_status::{Encodable, UiTransactionEncoding};
use tower_http::cors::CorsLayer;

use crate::{
    executor::{MAX_MEMPOOL_SIZE, MEMPOOL, STATE},
    node::BLOCK_DB,
    validator::TransactionValidator,
};

// JSON-RPC request structure
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

// JSON-RPC response structure
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

pub async fn start() {
    // Step1. create router
    let app = Router::new()
        .route("/", post(handle_rpc_request))
        .layer(CorsLayer::permissive());
    let addr = read_env_var("SEQUENCER_ADDR", "0.0.0.0:8898".to_owned());
    info!("Starting node rpc server on {:?}", addr);

    // Step2. start server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Main RPC handler
async fn handle_rpc_request(
    Json(request): Json<JsonRpcRequest>,
) -> Result<Json<JsonRpcResponse>, StatusCode> {
    info!("Received rpc request of method: {:?}", request.method);
    let response = match request.method.as_str() {
        "getHealth" => get_health(request.id).await,
        "getVersion" => get_version(request.id).await,
        "getAccountInfo" => get_account_info(request.id, request.params).await,
        "getBalance" => get_balance(request.id, request.params).await,
        "getLatestBlockhash" => get_latest_blockhash(request.id).await,
        "getFeeForMessage" => get_fee_for_message(request.id, request.params).await,
        "sendTransaction" => send_transaction(request.id, request.params).await,
        "simulateTransaction" => simulate_transaction(request.id, request.params).await,
        "getTransaction" => get_transaction(request.id, request.params).await,
        "getSignatureStatuses" => get_signature_statuses(request.id, request.params).await,
        "confirmTransaction" => confirm_transaction(request.id, request.params).await,
        "getTokenAccountsByOwner" => get_token_accounts_by_owner(request.id, request.params).await,
        "getMultipleAccounts" => get_multiple_accounts(request.id, request.params).await,
        _ => JsonRpcResponse {
            jsonrpc: request.jsonrpc,
            id: request.id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
            }),
        },
    };
    Ok(Json(response))
}

// Health check endpoint
async fn get_health(id: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!("ok")),
        error: None,
    }
}

// Version information
async fn get_version(id: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "solana-core": "1.18.0",
            "feature-set": 2891131721u32
        })),
        error: None,
    }
}

// Get account information
async fn get_account_info(id: Value, params: Option<Value>) -> JsonRpcResponse {
    let pubkey = match params
        .as_ref()
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|v| v.as_str())
    {
        Some(key) => key.to_string(),
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params".to_string(),
                }),
            };
        }
    };

    // Get balance from state
    let state_db = STATE.read().await;
    let balance = state_db.state.get_balance(&pubkey);

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "slot": 23816
            },
            "value": {
                "data": ["", "base64"],
                "executable": false,
                "lamports": balance,
                "owner": "11111111111111111111111111111111",
                "rentEpoch": 361
            }
        })),
        error: None,
    }
}

// Get balance
async fn get_balance(id: Value, params: Option<Value>) -> JsonRpcResponse {
    let pubkey = match params
        .as_ref()
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|v| v.as_str())
    {
        Some(key) => key.to_string(),
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params".to_string(),
                }),
            };
        }
    };

    // Get balance from state
    let state_db = STATE.read().await;
    let balance = state_db.state.get_balance(&pubkey);

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "apiVersion": "2.2.21",
                "slot": 23816
            },
            "value": balance
        })),
        error: None,
    }
}

// Get latest blockhash
async fn get_latest_blockhash(id: Value) -> JsonRpcResponse {
    let block_db = BLOCK_DB.read().await;
    // Use post_state_root
    let post_state_root = match block_db.cache.back() {
        Some(block) => block.post_state_root.unwrap_or_default(),
        None => [0u8; 32],
    };
    let simple_blockhash = bs58::encode(post_state_root).into_string();

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "apiVersion": "2.2.21",
                "slot": 32001
            },
            "value": {
                "blockhash": simple_blockhash,
                "lastValidBlockHeight": 33001
            }
        })),
        error: None,
    }
}

// Get Transaction
async fn get_transaction(id: Value, params: Option<Value>) -> JsonRpcResponse {
    // Extract transaction signature from params
    let signature: String = match params
        .as_ref()
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|v| v.as_str())
    {
        Some(sig) => sig.to_string(),
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params: transaction signature required".to_string(),
                }),
            };
        }
    };

    let block_db = BLOCK_DB.read().await;
    let txn_opt = block_db.search_txn(&signature);

    match txn_opt {
        Some(txn) => {
            // Use JsonParsed encoding to get the proper format with account objects
            let encoded_transaction = txn.encode(UiTransactionEncoding::JsonParsed);
            let formatted_transaction =
                serde_json::to_value(&encoded_transaction).unwrap_or_default();

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                  "blockTime": 1679123456,
                  "meta": {
                    "err": null,
                    "fee": 5000,
                    "innerInstructions": [],
                    "logMessages": [
                      "Program 11111111111111111111111111111111 invoke [1]",
                      "Program 11111111111111111111111111111111 success"
                    ],
                    "postBalances": [
                      0,
                      0,
                    ],
                    "postTokenBalances": [],
                    "preBalances": [
                      0,
                      0,
                    ],
                    "preTokenBalances": [],
                    "rewards": [],
                    "status": {
                      "Ok": null
                    }
                  },
                  "slot": 123456789,
                  "transaction": formatted_transaction,
                  "version": "legacy"
                })),
                error: None,
            }
        }
        None => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::Value::Null),
            error: None,
        },
    }
}

// Parse and decode transaction from RPC parameters
fn parse_and_decode_transaction(
    id: &Value,
    params: Option<Value>,
) -> Result<Transaction, JsonRpcResponse> {
    let (transaction_str, encoding) = match params.as_ref().and_then(|p| p.as_array()) {
        Some(arr) => {
            let tx_str = arr.get(0).and_then(|v| v.as_str()).unwrap_or("");
            let encoding = arr
                .get(1)
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("encoding"))
                .and_then(|v| v.as_str())
                .unwrap_or("base64");
            (tx_str.to_string(), encoding)
        }
        None => {
            return Err(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params".to_string(),
                }),
            });
        }
    };

    // Decode transaction
    let transaction_bytes = match encoding {
        "base64" => match general_purpose::STANDARD.decode(&transaction_str) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to decode base64 transaction: {}", e);
                return Err(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid base64 transaction: {}", e),
                    }),
                });
            }
        },
        "base58" => match bs58::decode(&transaction_str).into_vec() {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to decode base58 transaction: {}", e);
                return Err(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: format!("Invalid base58 transaction: {}", e),
                    }),
                });
            }
        },
        _ => {
            error!("Unsupported encoding: {}", encoding);
            return Err(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Unsupported encoding: {}", encoding),
                }),
            });
        }
    };

    let transaction: Transaction = match bincode::deserialize(&transaction_bytes) {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to deserialize transaction: {}", e);
            return Err(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid transaction format: {}", e),
                }),
            });
        }
    };

    Ok(transaction)
}

// Get fee for message
async fn get_fee_for_message(id: Value, _params: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "apiVersion": "2.2.21",
                "slot": 23816
            },
            "value": 5000
        })),
        error: None,
    }
}

// Send transaction
async fn send_transaction(id: Value, params: Option<Value>) -> JsonRpcResponse {
    // Parse and decode transaction using the common function
    let transaction = match parse_and_decode_transaction(&id, params) {
        Ok(tx) => tx,
        Err(error_response) => return error_response,
    };

    // Validate transaction
    match TransactionValidator::validate_transaction(true, &transaction).await {
        Ok(_) => {
            info!("Transaction validation passed");
        }
        Err(e) => {
            warn!("Transaction validation failed: {}", e);
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32003,
                    message: format!("Transaction validation failed: {}", e),
                }),
            };
        }
    }

    let signature = transaction.signatures[0].to_string();
    // Add transaction to mempool
    let mut mempool = MEMPOOL.write().await;
    if mempool.len() > MAX_MEMPOOL_SIZE {
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!(signature)),
            error: Some(JsonRpcError {
                code: -32602,
                message: "Mempool is full".to_string(),
            }),
        };
    }
    mempool.push(transaction);
    return JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!(signature)),
        error: None,
    };
}

// Simulate transaction
async fn simulate_transaction(id: Value, params: Option<Value>) -> JsonRpcResponse {
    // Parse and decode transaction using the common function
    let _transaction = match parse_and_decode_transaction(&id, params) {
        Ok(tx) => tx,
        Err(error_response) => return error_response,
    };

    return JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "apiVersion": "2.2.21",
                "slot": 23816
            },
            "value": {
                "err": null,
                "accounts": null,
                "logs": [
                    "Program 11111111111111111111111111111111 invoke [1]",
                    "Program 11111111111111111111111111111111 success"
                ],
                "returnData": null,
                "unitsConsumed": 150,
                "innerInstructions": [],
                "preBalances": [],
                "postBalances": [],
                "preTokenBalances": [],
                "postTokenBalances": []
            }
        })),
        error: None,
    };
}

// Get signature statuses
async fn get_signature_statuses(id: Value, params: Option<Value>) -> JsonRpcResponse {
    let signatures = match params
        .as_ref()
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|v| v.as_array())
    {
        Some(sigs) => sigs.clone(),
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params".to_string(),
                }),
            };
        }
    };

    let statuses: Vec<Value> = signatures
        .iter()
        .map(|_| {
            serde_json::json!({
                "slot": 23826,
                "confirmations": 10,
                "err": null,
                "status": {
                    "Ok": null
                },
                "confirmationStatus": "finalized"
            })
        })
        .collect();

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "apiVersion": "2.2.21",
                "slot": 23816
            },
            "value": statuses
        })),
        error: None,
    }
}

// Confirm transaction
async fn confirm_transaction(id: Value, params: Option<Value>) -> JsonRpcResponse {
    let _signature = match params
        .as_ref()
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|v| v.as_str())
    {
        Some(sig) => sig.to_string(),
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params".to_string(),
                }),
            };
        }
    };

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "apiVersion": "2.2.21",
                "slot": 23816
            },
            "value": {
                "confirmations": 10,
                "value": true
            }
        })),
        error: None,
    }
}

// Get token accounts by owner
async fn get_token_accounts_by_owner(id: Value, _params: Option<Value>) -> JsonRpcResponse {
    // No other tokens
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "apiVersion": "2.2.21",
                "slot": 23816
            },
            "value": []
        })),
        error: None,
    }
}

// Get multiple accounts
async fn get_multiple_accounts(id: Value, params: Option<Value>) -> JsonRpcResponse {
    let pubkeys = match params
        .as_ref()
        .and_then(|p| p.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|v| v.as_array())
    {
        Some(keys) => keys,
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params: expected array of public keys".to_string(),
                }),
            };
        }
    };

    // Convert pubkeys to strings
    let mut account_values = Vec::new();
    for pubkey_value in pubkeys {
        let pubkey = match pubkey_value.as_str() {
            Some(key) => key.to_string(),
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Invalid params: all public keys must be strings".to_string(),
                    }),
                };
            }
        };

        // Get balance from state
        let state_db = STATE.read().await;
        let balance = state_db.state.get_balance(&pubkey);
        let account_info = serde_json::json!({
            "data": ["", "base64"],
            "executable": false,
            "lamports": balance,
            "owner": "11111111111111111111111111111111",
            "rentEpoch": 361,
            "space": 0
        });
        account_values.push(account_info);
    }

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(serde_json::json!({
            "context": {
                "apiVersion": "2.2.21",
                "slot": 23816
            },
            "value": account_values
        })),
        error: None,
    }
}
