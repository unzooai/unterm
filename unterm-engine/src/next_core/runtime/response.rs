#![allow(dead_code)]

use super::dispatch::RuntimeDispatchResult;
use anyhow::{anyhow, Result};
use std::{
    fmt,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
};

pub(in crate::next_core) type RuntimeResponseResult = Result<RuntimeDispatchResult, String>;

pub(in crate::next_core) struct RuntimeResponseSender {
    tx: Sender<RuntimeResponseResult>,
}

pub(in crate::next_core) struct RuntimeResponseReceiver {
    rx: Receiver<RuntimeResponseResult>,
}

pub(in crate::next_core) fn channel() -> (RuntimeResponseSender, RuntimeResponseReceiver) {
    let (tx, rx) = mpsc::channel();
    (RuntimeResponseSender { tx }, RuntimeResponseReceiver { rx })
}

impl RuntimeResponseSender {
    pub(in crate::next_core) fn complete(self, result: Result<RuntimeDispatchResult>) {
        let _ = self.tx.send(result.map_err(|err| err.to_string()));
    }
}

impl RuntimeResponseReceiver {
    pub(in crate::next_core) fn try_recv(&self) -> Result<Option<RuntimeDispatchResult>> {
        match self.rx.try_recv() {
            Ok(result) => result.map(Some).map_err(|err| anyhow!(err)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err(anyhow!("runtime response channel closed before completion"))
            }
        }
    }

    pub(in crate::next_core) fn recv(self) -> Result<RuntimeDispatchResult> {
        self.rx
            .recv()
            .map_err(|err| anyhow!("runtime response channel closed: {err}"))?
            .map_err(|err| anyhow!(err))
    }
}

impl fmt::Debug for RuntimeResponseSender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeResponseSender")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_channel_round_trips_dispatch_result() {
        let (tx, rx) = channel();

        tx.complete(Ok(RuntimeDispatchResult::Unit));

        assert!(matches!(rx.recv().unwrap(), RuntimeDispatchResult::Unit));
    }

    #[test]
    fn response_channel_round_trips_dispatch_error_text() {
        let (tx, rx) = channel();

        tx.complete(Err(anyhow!("boom")));

        assert!(rx.recv().unwrap_err().to_string().contains("boom"));
    }

    #[test]
    fn response_try_recv_reports_empty_before_completion() {
        let (_tx, rx) = channel();

        assert!(rx.try_recv().unwrap().is_none());
    }
}
