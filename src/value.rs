use std::collections::HashMap;

use thiserror::Error;
use tokio::sync::{mpsc, oneshot, watch};
use std::time::Duration;
use tokio_util::time::{self, delay_queue};
use tokio;
use tokio_stream::StreamExt;
use leptos::logging::*;

use crate::user::UserId;

#[derive(Debug)]
enum Command {
    Update(UserId, String),
    GetCurrentValue(UserId, oneshot::Sender<String>),
    AwaitDifferentValue(UserId, String, oneshot::Sender<String>),
}

pub const DEFAULT_VALUE: &str = "";

#[derive(Debug, Clone, Error)]
pub enum ValueServiceError{
    #[error("worker shut down")]
    WorkerShutDown,
    #[error("worker did not respond")]
    WorkerDidNotRespond
}

struct ValueServiceWorker {
    command_rx: mpsc::Receiver<Command>,
    value_txs: HashMap<UserId, watch::Sender<String>>,
    expiration_entries: HashMap<UserId, delay_queue::Key>,
    expirations: time::DelayQueue<UserId>

}

impl ValueServiceWorker {
    fn new(command_rx: mpsc::Receiver<Command>) -> Self {
        Self {
            command_rx,
            value_txs: HashMap::<UserId, watch::Sender<String>>::new(),
            expiration_entries: HashMap::<UserId, delay_queue::Key>::new(),
            expirations: time::DelayQueue::<UserId>::new(),
        }
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        Command::Update(user_id, value) => self.handle_update(user_id, value),
                        Command::GetCurrentValue(user_id, sender) => self.handle_get_current_value(user_id, sender),
                        Command::AwaitDifferentValue(user_id, last_seen, sender) => self.handle_await_different_value(user_id, last_seen, sender),
                    }
                },
                Some(expiry) = self.expirations.next() => self.handle_expiry(expiry),
                else => {
                    log!("all channels have been closed, shutting down worker");
                    break;
                },
            }
        }
    }

    fn handle_update(&mut self, user_id: UserId, value: String) {
        log!("updating curr val of {} to {}", &user_id.0, &value);
        let tx = self.value_txs.entry(user_id.clone())
            .or_insert_with(|| { watch::channel::<String>(value.clone()).0 })
            .clone();

        tx.send_if_modified(|state| {
            if *state != value {
                log!("actually updating");
                *state = value.clone();
                true
            } else {
                log!("not actually updating");
                false
            }
        });

        if let Some(key) = self.expiration_entries.get(&user_id) {
            log!("removing expiration entry");
            self.expirations.reset(&key, Duration::from_secs(5));
        } else {
            let delay_key = self.expirations.insert(user_id.clone(), Duration::from_secs(5));
            self.expiration_entries.insert(user_id, delay_key);
        }
    }

    fn handle_get_current_value(&self, user_id: UserId, sender: oneshot::Sender<String>) {
        log!("getting curr val of {}", &user_id.0);
        let value = self.value_txs.get(&user_id)
            .map(|tx| tx.subscribe().borrow().clone())
            .unwrap_or("".to_string());
        log!("current value is {:?}", &value);
        sender.send(value).unwrap_or_else(|_| warn!("client disconnected"));
    }

    fn handle_await_different_value(&mut self, user_id: UserId, last_seen: String, sender: oneshot::Sender<String>) {
        let tx = self.value_txs.entry(user_id.clone())
            .or_insert_with(|| { watch::channel::<String>("".to_string()).0 })
            .clone();
        
        tokio::spawn(async move {
            let mut rx = tx.subscribe();
            loop {
                let current = rx.borrow_and_update().clone();
                if current != last_seen {
                    log!("received different value {:?}", &current);
                    sender.send(current).unwrap_or_else(|_| warn!("client disconnected"));
                    break;
                }
                log!("received x {:?}", &current);
                rx.changed().await.unwrap_or_else(|_| warn!("channel closed"));
            }
        });
    }

    fn handle_expiry(&mut self, expiry: delay_queue::Expired<UserId>) {
        let uid = expiry.into_inner();
        self.expiration_entries.remove(&uid);
        match self.value_txs.get(&uid) {
            Some(tx) => {
                log!("expiring value for {}", &uid.0);
                tx.send_modify(|state| *state = "".to_string());
                if tx.receiver_count() == 0 {
                    self.value_txs.remove(&uid);
                }
            },
            None => warn!("tried to expire value for uid {:?}, but no value was found, skipping.", &uid),
        }
    }
}

pub struct ValueService {
    command_tx: mpsc::Sender<Command>,
}

impl ValueService {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel(100);
        let worker = ValueServiceWorker::new(command_rx);
        tokio::spawn(worker.run());
        Self { command_tx }
    }

    pub async fn update(&self, user_id: UserId, value: String) -> Result<(), ValueServiceError> {
        self.command_tx.send(Command::Update(user_id, value)).await.map_err(|_| ValueServiceError::WorkerShutDown)
    }

    pub async fn get_current_value(&self, user_id: UserId) -> Result<String, ValueServiceError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(Command::GetCurrentValue(user_id, tx)).await.map_err(|_| ValueServiceError::WorkerShutDown)?;
        rx.await.map_err(|_| ValueServiceError::WorkerDidNotRespond)
    }

    pub async fn await_different_value(&self, user_id: UserId, last_seen: String) -> Result<String, ValueServiceError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(Command::AwaitDifferentValue(user_id, last_seen, tx)).await.map_err(|_| ValueServiceError::WorkerShutDown)?;
        rx.await.map_err(|_| ValueServiceError::WorkerDidNotRespond)
    }
}
