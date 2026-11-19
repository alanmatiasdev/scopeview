//! A one-slot hand-off between the capture thread and the decode thread.
//!
//! The capture thread must always drain the driver's buffer queue quickly: if it waits for a
//! slow decode, the queue fills and every frame shown is several frames old. So it drops its
//! frame here and moves on; if the decoder has not picked up the previous one, that one is
//! replaced. Latency never builds up, whatever the decode speed.

use std::sync::{Condvar, Mutex, MutexGuard};

pub(crate) struct Mailbox<T> {
    state: Mutex<State<T>>,
    ready: Condvar,
}

struct State<T> {
    item: Option<T>,
    closed: bool,
}

impl<T> Mailbox<T> {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(State {
                item: None,
                closed: false,
            }),
            ready: Condvar::new(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Store `item`, replacing any item nobody took yet.
    pub(crate) fn put(&self, item: T) {
        self.lock().item = Some(item);
        self.ready.notify_one();
    }

    /// Wake the consumer for good: `take` returns `None` once the slot is empty.
    pub(crate) fn close(&self) {
        self.lock().closed = true;
        self.ready.notify_all();
    }

    /// Wait for the newest item. `None` means the mailbox was closed.
    pub(crate) fn take(&self) -> Option<T> {
        let mut state = self.lock();
        loop {
            if let Some(item) = state.item.take() {
                return Some(item);
            }
            if state.closed {
                return None;
            }
            state = self
                .ready
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn newest_item_wins_and_nothing_builds_up() {
        let mailbox = Mailbox::new();
        mailbox.put(1);
        mailbox.put(2);
        mailbox.put(3);
        assert_eq!(mailbox.take(), Some(3));
        mailbox.close();
        assert_eq!(mailbox.take(), None);
    }

    #[test]
    fn take_blocks_until_an_item_arrives() {
        let mailbox = Arc::new(Mailbox::new());
        let consumer = {
            let mailbox = Arc::clone(&mailbox);
            thread::spawn(move || mailbox.take())
        };
        thread::sleep(std::time::Duration::from_millis(30));
        mailbox.put("frame");
        assert_eq!(consumer.join().unwrap(), Some("frame"));
    }

    #[test]
    fn close_wakes_a_waiting_consumer() {
        let mailbox: Arc<Mailbox<u8>> = Arc::new(Mailbox::new());
        let consumer = {
            let mailbox = Arc::clone(&mailbox);
            thread::spawn(move || mailbox.take())
        };
        thread::sleep(std::time::Duration::from_millis(30));
        mailbox.close();
        assert_eq!(consumer.join().unwrap(), None);
    }

    #[test]
    fn an_item_put_before_close_is_still_delivered() {
        let mailbox = Mailbox::new();
        mailbox.put(7);
        mailbox.close();
        assert_eq!(mailbox.take(), Some(7));
        assert_eq!(mailbox.take(), None);
    }
}
