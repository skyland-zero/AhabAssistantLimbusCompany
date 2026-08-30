//! Coalesced transport-to-UI wakeups.
//!
//! The websocket remains push based. This channel only tells the GPUI task
//! that transport-owned queues have work, so bursts need a single wakeup.

#[derive(Clone)]
pub(crate) struct TransportActivity {
    sender: async_channel::Sender<()>,
    receiver: async_channel::Receiver<()>,
}

impl TransportActivity {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = async_channel::bounded(1);
        Self { sender, receiver }
    }

    pub(crate) fn notify(&self) {
        let _ = self.sender.try_send(());
    }

    pub(crate) async fn wait(&self) -> bool {
        self.receiver.recv().await.is_ok()
    }

    #[cfg(test)]
    pub(crate) fn try_take(&self) -> bool {
        self.receiver.try_recv().is_ok()
    }
}

impl Default for TransportActivity {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_notifications_are_coalesced() {
        let activity = TransportActivity::new();

        activity.notify();
        activity.notify();

        assert!(activity.try_take());
        assert!(!activity.try_take());
    }
}
