//! External chaincode (chaincode-as-a-service) support.
//!
//! Allows chaincode to run as an external HTTP service instead of in-process Wasm.

use serde::{Deserialize, Serialize};

use super::ChaincodeError;

/// Runtime mode for a chaincode definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChaincodeRuntime {
    /// In-process Wasm execution.
    Wasm {
        fuel_limit: u64,
        memory_limit: Option<usize>,
    },
    /// External HTTP service.
    External { endpoint: String, tls: bool },
}

impl Default for ChaincodeRuntime {
    fn default() -> Self {
        Self::Wasm {
            fuel_limit: 10_000_000,
            memory_limit: None,
        }
    }
}

/// Client for invoking an external chaincode service via HTTP.
pub struct ExternalChaincodeClient {
    endpoint: String,
    tls: bool,
}

/// Request body sent to the external chaincode service.
#[derive(Serialize)]
struct InvokeRequest<'a> {
    function: &'a str,
    args: &'a [&'a str],
    state_context: &'a str,
}

/// Response body from the external chaincode service.
#[derive(Deserialize)]
struct InvokeResponse {
    result: Option<String>,
    error: Option<String>,
}

impl ExternalChaincodeClient {
    pub fn new(endpoint: &str, tls: bool) -> Result<Self, ChaincodeError> {
        if endpoint.is_empty() {
            return Err(ChaincodeError::Execution("empty endpoint".into()));
        }
        Ok(Self {
            endpoint: endpoint.to_string(),
            tls,
        })
    }

    /// Invoke the external chaincode.
    ///
    /// Sends HTTP POST to `{endpoint}/invoke` with JSON body.
    pub async fn invoke(
        &self,
        function: &str,
        args: &[&str],
        state_context: &str,
    ) -> Result<Vec<u8>, ChaincodeError> {
        let scheme = if self.tls { "https" } else { "http" };
        let url = format!("{scheme}://{}/invoke", self.endpoint);

        let body = InvokeRequest {
            function,
            args,
            state_context,
        };

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChaincodeError::Execution(format!("HTTP request failed: {e}")))?;

        let status = resp.status();
        let resp_body: InvokeResponse = resp
            .json()
            .await
            .map_err(|e| ChaincodeError::Execution(format!("failed to parse response: {e}")))?;

        if !status.is_success() {
            return Err(ChaincodeError::Execution(
                resp_body.error.unwrap_or_else(|| format!("HTTP {status}")),
            ));
        }

        Ok(resp_body.result.unwrap_or_default().into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaincode_runtime_default_is_wasm() {
        let rt = ChaincodeRuntime::default();
        assert!(matches!(rt, ChaincodeRuntime::Wasm { .. }));
    }

    #[test]
    fn wasm_runtime_serde_roundtrip() {
        let rt = ChaincodeRuntime::Wasm {
            fuel_limit: 5_000_000,
            memory_limit: Some(1024 * 1024),
        };
        let json = serde_json::to_string(&rt).unwrap();
        let decoded: ChaincodeRuntime = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, decoded);
    }

    #[test]
    fn external_runtime_serde_roundtrip() {
        let rt = ChaincodeRuntime::External {
            endpoint: "chaincode.example.com:9999".into(),
            tls: true,
        };
        let json = serde_json::to_string(&rt).unwrap();
        let decoded: ChaincodeRuntime = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, decoded);
    }

    #[test]
    fn external_client_rejects_empty_endpoint() {
        let result = ExternalChaincodeClient::new("", false);
        assert!(result.is_err());
    }

    #[test]
    fn external_client_creates_with_valid_endpoint() {
        let client = ExternalChaincodeClient::new("localhost:9999", false).unwrap();
        assert_eq!(client.endpoint, "localhost:9999");
        assert!(!client.tls);
    }
}
