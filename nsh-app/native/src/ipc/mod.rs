//! The transport-independent JSON-RPC boundary.
//!
//! The state layer depends on [`RpcGateway`] and the contract, never on the
//! sidecar socket or the mock. [`BackendClient`] selects the transport.

pub mod backend;
pub mod bridge;
pub mod contract;
pub mod gateway;
pub mod mock;
pub mod websocket;

pub use backend::{BackendClient, decode_response};
pub use contract::{
    EventEnvelope, RequestId, RequestSequence, RpcCompletion, RpcError, RpcRequest, RpcResponse,
};
pub use gateway::RpcGateway;
pub use mock::MockClient;

/// Transport-independent request boundary used by tests and by the gateway.
pub trait RpcClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse;
}
