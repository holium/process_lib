use crate::{Message, PackageId, Request};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// "package_id" is the package. Capabilities are checked, you can access another process's
/// database if it has given you the capability.
#[derive(Debug, Serialize, Deserialize)]
pub struct PythonRequest {
    pub package_id: PackageId,
    pub action: PythonAction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PythonAction {
    RunScript {
        /// The script to run must be in the package's `scripts` directory
        script: String,
        /// The function to call in the script
        func: String,
        /// The arguments to pass to the script
        args: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PythonResponse {
    Result { data: Vec<u8> },
    Err { error: PythonError },
}

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum PythonError {
    #[error("python: No capability: {error}")]
    NoCap { error: String },
    #[error("python: input bytes/json/key error: {error}")]
    InputError { error: String },
    #[error("python: IO error: {error}")]
    IOError { error: String },
}

/// Python runner helper.
/// You can call it's impl functions to interact with it.
pub struct Python {
    pub package_id: PackageId,
}

/// Process lib for python.
/// This is a helper struct for python.
///
/// Functions:
///     run_script(script: String, args: Vec<String>)
impl Python {
    /// Create a new python runner.
    pub fn new(package_id: PackageId) -> Self {
        Self { package_id }
    }
    /// Run a python script with arguments.
    /// The script to run must be in the package's `scripts` directory.
    pub fn run_script(
        &self,
        script: String,
        func: String,
        args: Vec<String>,
    ) -> anyhow::Result<Vec<u8>> {
        let res = Request::new()
            .target(("our", "python", "distro", "sys"))
            .body(serde_json::to_vec(&PythonRequest {
                package_id: self.package_id.clone(),
                action: PythonAction::RunScript { script, func, args },
            })?)
            .send_and_await_response(5)
            .map_err(|e| anyhow::anyhow!("python: {}", e))?;

        match res {
            Ok(Message::Response { body, .. }) => {
                let response = serde_json::from_slice::<PythonResponse>(&body)?;

                match response {
                    PythonResponse::Result { data } => Ok(data),
                    PythonResponse::Err { error } => Err(error.into()),
                }
            }
            _ => Err(anyhow::anyhow!("python: unexpected message: {:?}", res)),
        }
    }
}
