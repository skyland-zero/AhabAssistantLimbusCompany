#![allow(dead_code, unused_imports)]

pub mod backend;
pub mod contract;
pub mod gateway;
pub mod mock;
pub mod websocket;

use contract::RpcRequest;

/// Transport-independent JSON-RPC boundary. The GPUI UI should depend only on
/// this trait; `BackendClient` selects the real sidecar or an explicit mock.
pub trait RpcClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse;
}

pub use backend::{BackendClient, decode_response};
pub use contract::{EventEnvelope, RequestId, RequestSequence, RpcError, RpcResponse};
pub use gateway::RpcGateway;
pub use mock::MockClient;
