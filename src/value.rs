use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot, watch};
use std::time::Duration;
use tokio_util::time::{self, delay_queue};
use tokio;
use tokio_stream::StreamExt;
use leptos::{logging::*, prelude::ServerFnError};

use crate::user::UserId;

#[derive(Debug)]
enum Command {
    Update(UserId, String),
    GetCurrentValue(UserId, oneshot::Sender<String>),
    AwaitDifferentValue(UserId, String, oneshot::Sender<String>),
}

pub const DEFAULT_VALUE: &str = "";

pub struct ValueService {
    command_tx: mpsc::Sender<Command>,
}

impl ValueService {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel(100);
        Self::run(command_rx);
        Self { command_tx }
    }

    fn run(mut command_rx: mpsc::Receiver<Command>) {

        tokio::spawn(async move {

            let mut value_txs = HashMap::<UserId, watch::Sender<String>>::new();

            let mut expiration_entries = HashMap::<UserId, delay_queue::Key>::new();
            let mut expirations = time::DelayQueue::<UserId>::new();

            loop {
                tokio::select! {
                    Some(command) = command_rx.recv() => {
                        match command {
                            Command::Update(user_id, value) => {
                                log!("updating curr val of {} to {}", &user_id.0, &value);
                                let tx = value_txs.entry(user_id.clone()).or_insert_with(|| { watch::channel::<String>(value.clone()).0 }).clone();
                                tx.send_if_modified(|state| {
                                    if *state != value.clone() {
                                        log!("actually updating");
                                        *state = value.clone();
                                        return true;
                                    }
                                    log!("not actually updating");
                                    return false;
                                });

                                if let Some(key) = expiration_entries.get(&user_id) {
                                    log!("removing expiration entry");
                                    expirations.reset(&key, Duration::from_secs(5));
                                } else {
                                    let delay_key = expirations.insert(user_id.clone(), Duration::from_secs(5));
                                    expiration_entries.insert(user_id, delay_key);
                                }
                            },
                            Command::GetCurrentValue(user_id, sender) => {
                                log!("getting curr val of {}", &user_id.0);
                                let value: String = value_txs.get(&user_id).map(|tx| tx.subscribe().borrow().clone()).unwrap_or("".to_string());
                                log!("current value is {:?}", &value);
                                sender.send(value).unwrap_or_else(|_| {
                                    warn!("unable to respond to get_current_value, receiver dropped");
                                });
                            },
                            Command::AwaitDifferentValue(user_id, last_seen, sender) => {
                                let tx = value_txs.entry(user_id.clone()).or_insert_with(|| { watch::channel::<String>("".to_string()).0 }).clone();
                                tokio::spawn(async move {
                                    let mut rx = tx.subscribe();

                                    loop {
                                        let current = rx.borrow_and_update().clone();
                                        // if let None = current {
                                        //     log!("received None");
                                        //     break;
                                        // } else if Some(current) != last_seen {
                                        //     sender.send(sender);
                                        //     break;
                                        // }

                                        match current {
                                            c if c != last_seen => {
                                                log!("received different value {:?}", &c);
                                                sender.send(c).unwrap_or_else(|_| {
                                                    warn!("unable to respond to await_different_value, receiver dropped");
                                                });
                                                break;
                                            },
                                            x => {
                                                log!("received x {:?}", &x);
                                            }
                                        }
                                        
                                        // Wait for the next update.
                                        // If an update happened between the check above and this line,
                                        // .changed() resolves immediately.
                                        rx.changed().await.unwrap_or_else(|_| {
                                            warn!("channel closed");
                                        });
                                    }
                                });
                            },
                        }

                    },
                    Some(expiry) = expirations.next() => {
                        let uid: UserId = expiry.into_inner();
                        expiration_entries.remove(&uid);
                        match value_txs.get(&uid) {
                            Some(tx) => {
                                log!("expiring value for {}", &uid.0);
                                tx.send_modify(|state| {
                                    *state = "".to_string();
                                });
                                if tx.receiver_count() == 0 {
                                    value_txs.remove(&uid);
                                }
                            },
                            None => warn!("tried to expire value for uid {:?}, but no value was found, skipping.", &uid),
                        }
                    }
                    else => { break }
                };
            }
        });
    }

    pub async fn update(&self, user_id: UserId, value: String) {
        self.command_tx.send(Command::Update(user_id, value)).await.unwrap_or_else(|_| {
            warn!("failed to send update command to value service, receiver was dropped");
        });
    }

    pub async fn get_current_value(&self, user_id: UserId) -> Result<String, ServerFnError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(Command::GetCurrentValue(user_id, tx)).await.unwrap_or_else(|_| {
            warn!("failed to send get_current_value command to value service, receiver was dropped");
        });
        rx.await.map_err(|_| ServerFnError::new("failed to wait for current value, sender was dropped"))
    }
    pub async fn await_different_value(&self, user_id: UserId, last_seen: String) -> Result<String, ServerFnError> {
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(Command::AwaitDifferentValue(user_id, last_seen, tx)).await.unwrap_or_else(|_| {
            warn!("failed to send await_different_value command to value service, receiver was dropped");
        });
        rx.await.map_err(|_| ServerFnError::new("failed to wait for different value, sender was dropped"))
    }
}
