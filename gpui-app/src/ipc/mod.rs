#![allow(dead_code, unused_imports)]

pub mod contract;
pub mod mock;
pub mod websocket;

use contract::RpcRequest;

/// Transport-independent JSON-RPC boundary. The GPUI UI should depend only on
/// this trait; the hybrid client can route migrated methods to the Python
/// sidecar while the remaining pages continue to use MockClient.
pub trait RpcClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse;
}

pub use contract::{EventEnvelope, RequestId, RequestSequence, RpcError, RpcResponse};
pub use mock::MockClient;
