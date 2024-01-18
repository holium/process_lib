use crate::{get_blob, Message, PackageId, Request};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Actions are sent to a specific graphdb database, "db" is the name,
/// "package_id" is the package. Capabilities are checked, you can access another process's
/// database if it has given you the capability.
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphDbRequest {
    pub package_id: PackageId,
    pub db: String,
    pub action: GraphDbAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GraphDbAction {
    Open,
    RemoveDb,
    Create { statement: String },
    Update { statement: String },
    Delete { record_id: String },
    Read { query: String },
    Backup,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GraphDbResponse {
    Ok,
    Data,
    Err { error: GraphDbError },
}

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum GraphDbError {
    #[error("graphdb: DbDoesNotExist")]
    NoDb,
    #[error("graphdb: KeyNotFound")]
    KeyNotFound,
    #[error("graphdb: no Tx found")]
    NoTx,
    #[error("graphdb: No capability: {error}")]
    NoCap { error: String },
    #[error("graphdb: surrealdb internal error: {error}")]
    SurrealDBError { action: String, error: String },
    #[error("graphdb: input bytes/json/key error: {error}")]
    InputError { error: String },
    #[error("graphdb: IO error: {error}")]
    IOError { error: String },
}

/// GraphDb helper struct for a db.
/// Opening or creating a db will give you a Result<GraphDb>.
/// You can call it's impl functions to interact with it.
pub struct GraphDb {
    pub package_id: PackageId,
    pub db: String,
}

impl GraphDb {
    /// Query database. Only allows graphdb read keywords.
    pub fn read(
        &self,
        query: String,
        params: Vec<String>,
    ) -> anyhow::Result<Vec<HashMap<String, serde_json::Value>>> {
        let res = Request::new()
            .target(("our", "graphdb", "distro", "sys"))
            .body(serde_json::to_vec(&GraphDbRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: GraphDbAction::Read { query },
            })?)
            .blob_bytes(serde_json::to_vec(&params)?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<GraphDbResponse>(&body)?;

                match response {
                    GraphDbResponse::Data => {
                        let blob = get_blob().ok_or_else(|| GraphDbError::InputError {
                            error: "graphdb: no blob".to_string(),
                        })?;
                        let values = serde_json::from_slice::<
                            Vec<HashMap<String, serde_json::Value>>,
                        >(&blob.bytes)
                        .map_err(|e| GraphDbError::InputError {
                            error: format!("graphdb: gave unparsable response: {}", e),
                        })?;
                        Ok(values)
                    }
                    GraphDbResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "graphdb: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => Err(anyhow::anyhow!("graphdb: unexpected message: {:?}", res)),
        }
    }

    /// Execute a statement. Only allows graphdb write keywords.
    pub fn create(&self, statement: String, params: Vec<serde_json::Value>) -> anyhow::Result<()> {
        let res = Request::new()
            .target(("our", "graphdb", "distro", "sys"))
            .body(serde_json::to_vec(&GraphDbRequest {
                package_id: self.package_id.clone(),
                db: self.db.clone(),
                action: GraphDbAction::Create { statement },
            })?)
            .blob_bytes(serde_json::to_vec(&params)?)
            .send_and_await_response(5)?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<GraphDbResponse>(&body)?;

                match response {
                    GraphDbResponse::Ok => Ok(()),
                    GraphDbResponse::Err { error } => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "graphdb: unexpected response {:?}",
                        response
                    )),
                }
            }
            _ => Err(anyhow::anyhow!("graphdb: unexpected message: {:?}", res)),
        }
    }
}

/// Open or create graphdb database.
pub fn open(package_id: PackageId, db: &str) -> anyhow::Result<GraphDb> {
    let res = Request::new()
        .target(("our", "graphdb", "distro", "sys"))
        .body(serde_json::to_vec(&GraphDbRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: GraphDbAction::Open,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<GraphDbResponse>(&body)?;

            match response {
                GraphDbResponse::Ok => Ok(GraphDb {
                    package_id,
                    db: db.to_string(),
                }),
                GraphDbResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!(
                    "graphdb: unexpected response {:?}",
                    response
                )),
            }
        }
        _ => Err(anyhow::anyhow!("graphdb: unexpected message: {:?}", res)),
    }
}

/// Remove and delete graphdb database.
pub fn remove_db(package_id: PackageId, db: &str) -> anyhow::Result<()> {
    let res = Request::new()
        .target(("our", "graphdb", "distro", "sys"))
        .body(serde_json::to_vec(&GraphDbRequest {
            package_id: package_id.clone(),
            db: db.to_string(),
            action: GraphDbAction::RemoveDb,
        })?)
        .send_and_await_response(5)?;

    match res {
        Ok(Message::Response { body, .. }) => {
            let response = serde_json::from_slice::<GraphDbResponse>(&body)?;

            match response {
                GraphDbResponse::Ok => Ok(()),
                GraphDbResponse::Err { error } => Err(error.into()),
                _ => Err(anyhow::anyhow!(
                    "graphdb: unexpected response {:?}",
                    response
                )),
            }
        }
        _ => Err(anyhow::anyhow!("graphdb: unexpected message: {:?}", res)),
    }
}
