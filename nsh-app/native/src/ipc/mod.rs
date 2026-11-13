//! The transport-independent JSON-RPC boundary.
//!
//! The state layer depends on [`RpcGateway`] and the contract, never on the
//! sidecar socket or the mock. [`BackendClient`] selects the transport. The
//! contract itself lives in the `aalc-contract` crate, re-exported here as
//! [`contract`], so it builds without the transport or `nsh-core`.

pub mod backend;
pub mod bridge;
pub mod gateway;
pub mod mock;
pub mod websocket;

pub use aalc_contract as contract;

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
