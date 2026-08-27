use super::{
    RpcClient,
    contract::{RpcError, RpcRequest, RpcResponse},
};

/// Reserved transport seam for the Python sidecar. Networking is intentionally
/// deferred in M0; callers can still handle a structured unavailable error.
pub struct WebSocketClient;

impl WebSocketClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSocketClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcClient for WebSocketClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse {
        RpcResponse::failure(
            request.id,
            RpcError::new(-32000, "websocket transport is not implemented"),
        )
    }
}
