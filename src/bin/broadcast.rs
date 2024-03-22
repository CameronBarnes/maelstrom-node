use std::{
    collections::HashMap,
    io::StdoutLock,
    sync::{atomic::AtomicBool, mpsc::Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::Result;
use maelstrom_node::{main_loop, Body, Event, Message, Node};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Callback {
    pub nodes: Vec<String>,
    uid: String,
}

impl Callback {
    pub fn id(&self) -> &String {
        &self.uid
    }
}

impl Default for Callback {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            uid: generate_unique_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Payload {
    Broadcast {
        message: usize,
        #[serde(default)]
        callback: Callback,
    },
    BroadcastOk {},
    Read {},
    ReadOk {
        messages: Vec<usize>,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk {},
}

enum InjectedPayload {
    CallbackTimeout,
}

struct BroadcastNode {
    id: String,
    neighbours: Vec<String>,
    msg_id: usize,
    messages: Vec<usize>,
    seen_uids: Vec<String>,
    callbacks: Vec<(usize, String, usize, String)>,
    join_handle: Option<JoinHandle<()>>,
}

impl BroadcastNode {
    fn broadcast(
        &mut self,
        msg: Message<Payload>,
        message: usize,
        callback: Callback,
        output: &mut StdoutLock,
    ) -> Result<()> {
        for id in self.neighbours.clone() {
            if callback.nodes.contains(&id) {
                continue;
            }
            let msg = msg.clone();
            let mut msg = self.reply(
                msg,
                Payload::Broadcast {
                    message,
                    callback: callback.clone(),
                },
            );
            self.callbacks.push((
                message,
                id.clone(),
                msg.body.id.unwrap(),
                callback.id().clone(),
            ));
            msg.dest = id.clone();
            msg.send(output)?;
        }

        Ok(())
    }

    pub fn join_thread(&mut self) {
        let handle = self.join_handle.take();
        let _ = handle
            .expect("this function should only be called once")
            .join(); // FIXME: this
                     // might be a
                     // problem
    }
}

fn generate_unique_id() -> String {
    ulid::Ulid::new().to_string()
}

static EOF_FLAG: AtomicBool = AtomicBool::new(false);

impl Node<Payload, InjectedPayload> for BroadcastNode {
    fn from_init(
        init: maelstrom_node::Init,
        tx: Sender<Event<Payload, InjectedPayload>>,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        let join_handle = Some(thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(300));
            if EOF_FLAG.load(std::sync::atomic::Ordering::Acquire)
                || tx
                    .send(Event::Injected(InjectedPayload::CallbackTimeout))
                    .is_err()
            {
                break;
            }
        }));

        let mut neighbours = init.node_ids;
        neighbours.retain_mut(|id| (*id).ne(&init.node_id));
        Ok(Self {
            id: init.node_id,
            neighbours,
            msg_id: 1,
            messages: Vec::new(),
            seen_uids: Vec::new(),
            callbacks: Vec::new(),
            join_handle,
        })
    }

    fn next_msg_id(&mut self) -> usize {
        let out = self.msg_id;
        self.msg_id += 1;
        out
    }

    fn node_id(&self) -> String {
        self.id.clone()
    }

    fn process_message(
        &mut self,
        event: Event<Payload, InjectedPayload>,
        output: &mut StdoutLock,
    ) -> Result<()> {
        match event {
            Event::Message(msg) => match msg.body.payload.clone() {
                Payload::Broadcast {
                    message,
                    mut callback,
                } => {
                    if !self.seen_uids.contains(callback.id()) {
                        callback.nodes.push(self.id.clone());
                        self.messages.push(message);
                        self.seen_uids.push(callback.id().clone());
                        self.broadcast(msg.clone(), message, callback.clone(), &mut *output)?;
                    }
                    self.reply(msg, Payload::BroadcastOk {}).send(output)?;
                }
                Payload::Read {} => {
                    self.reply(
                        msg,
                        Payload::ReadOk {
                            messages: self.messages.clone(),
                        },
                    )
                    .send(output)?;
                }
                Payload::Topology { topology } => {
                    if let Some(neighbours) = topology.get(&self.id) {
                        self.neighbours = neighbours.to_owned();
                    }
                    self.reply(msg, Payload::TopologyOk {}).send(output)?;
                }
                Payload::BroadcastOk {} => self.callbacks.retain(|(_msg, node, msg_id, _uid)| {
                    msg.src.ne(node)
                        && !msg
                            .body
                            .reply_to
                            .is_some_and(|reply_id| reply_id == *msg_id)
                }),
                Payload::TopologyOk { .. } | Payload::ReadOk { .. } => {}
            },
            Event::Injected(_) => {
                let callbacks: Vec<(usize, String, usize, String)> = self.callbacks.drain(..).collect();
                for (message, node, _original_msg_id, uid) in callbacks {
                    let callback = Callback {
                        nodes: vec!(node.clone()),
                        uid: uid.clone()
                    };
                    let msg = Message::new(
                        self.id.clone(),
                        node.clone(),
                        Body::new(
                            Some(self.next_msg_id()),
                            Payload::Broadcast { message, callback },
                        ),
                    );
                    msg.send(output)?;
                    self.callbacks.push((message, node, msg.body.id.unwrap(), uid));
                }
            }
            Event::EOF => {
                EOF_FLAG.store(true, std::sync::atomic::Ordering::Release);
                self.join_thread();
            }
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<BroadcastNode, Payload, InjectedPayload>()
}
