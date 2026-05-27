use std::sync::Arc;

use tokio::sync::watch;

#[derive(Clone)]
pub struct BroadcastSender<T: Clone + Send + Sync + 'static> {
    inner: Arc<watch::Sender<Option<T>>>,
}

#[derive(Clone)]
pub struct BroadcastReceiver<T: Clone + Send + Sync + 'static> {
    rx: watch::Receiver<Option<T>>,
}

pub fn broadcast_oneshot<T: Clone + Send + Sync + 'static>()
-> (BroadcastSender<T>, BroadcastReceiver<T>) {
    let (tx, rx) = watch::channel(None);
    (
        BroadcastSender {
            inner: Arc::new(tx),
        },
        BroadcastReceiver { rx },
    )
}

impl<T: Clone + Send + Sync + 'static> BroadcastSender<T> {
    pub fn send(&self, value: T) {
        let _ = self.inner.send(Some(value));
    }

    pub fn is_set(&self) -> bool {
        self.inner.borrow().is_some()
    }

    pub fn subscribe(&self) -> BroadcastReceiver<T> {
        BroadcastReceiver {
            rx: self.inner.subscribe(),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> BroadcastReceiver<T> {
    pub async fn recv(&mut self) -> T {
        if let Some(v) = self.rx.borrow().clone() {
            return v;
        }
        loop {
            if self.rx.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
            if let Some(v) = self.rx.borrow().clone() {
                return v;
            }
        }
    }

    pub fn try_recv(&self) -> Option<T> {
        self.rx.borrow().clone()
    }
}
