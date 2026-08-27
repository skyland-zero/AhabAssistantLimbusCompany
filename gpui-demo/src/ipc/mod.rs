pub mod contract;
pub mod mock;
pub mod websocket;

use contract::{RpcRequest, RpcResponse};

/// Transport-independent JSON-RPC boundary. The GPUI UI should depend only on
/// this trait; MockClient and the future sidecar transport share the contract.
pub trait RpcClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse;
}

pub use contract::{EventEnvelope, RequestId, RequestSequence, RpcError};
pub use mock::MockClient;
