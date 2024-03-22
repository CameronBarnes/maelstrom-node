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
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Payload {
    Broadcast {
        message: usize,
        check: Option<bool>,
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
    Gossip {
        messages: Vec<usize>,
    },
    GossipOk {
        messages: Vec<usize>,
    },
}

enum InjectedPayload {
    GossipTrigger,
}

struct BroadcastNode {
    id: String,
    neighbours: Vec<String>,
    msg_id: usize,
    messages: Vec<usize>,
    has_update: Option<String>,
    join_handle: Option<JoinHandle<()>>,
}

impl BroadcastNode {
    fn broadcast(
        &mut self,
        msg: Message<Payload>,
        message: usize,
        output: &mut StdoutLock,
    ) -> Result<()> {
        for id in self.neighbours.clone() {
            let msg = msg.clone();
            let mut msg = self.reply(
                msg,
                Payload::Broadcast {
                    message,
                    check: Some(true),
                },
            );
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
            std::thread::sleep(Duration::from_millis(20));
            if EOF_FLAG.load(std::sync::atomic::Ordering::Acquire)
                || tx
                    .send(Event::Injected(InjectedPayload::GossipTrigger))
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
            has_update: None,
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
                Payload::Broadcast { message, check } => {
                    if check.is_none() {
                        self.broadcast(msg.clone(), message, &mut *output)?;
                    }
                    if !self.messages.contains(&message) {
                        self.messages.push(message);
                        self.has_update = Some(msg.src.clone());
                    }
                    if msg.src.starts_with('c') {
                        self.reply(msg, Payload::BroadcastOk {}).send(output)?;
                    }
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
                Payload::BroadcastOk {} | Payload::TopologyOk { .. } | Payload::ReadOk { .. } => {}
                Payload::Gossip { messages } => {
                    let mut to_return = self.messages.clone();
                    to_return.retain(|msg| !messages.contains(msg));
                    let src = msg.src.clone();
                    self.reply(msg, Payload::GossipOk{messages: to_return}).send(output)?;
                    let mut flag = false;
                    for msg in messages {
                        if !self.messages.contains(&msg) {
                            flag = true;
                            self.messages.push(msg);
                        }
                    }
                    if flag {
                        self.has_update = Some(src);
                    }
                },
                Payload::GossipOk { messages } => {
                    let mut flag = false;
                    for msg in messages {
                        if !self.messages.contains(&msg) {
                            flag = true;
                            self.messages.push(msg);
                        }
                    }
                    if flag {
                        self.has_update = Some(msg.src);
                    }
                },
            },
            Event::Injected(_) => {
                let Some(last) = self.has_update.take() else {
                    return Ok(());
                };
                for node in self.neighbours.clone() {
                    if node.eq(&self.id) || node.eq(&last) {
                        continue;
                    }
                    let msg = Message::new(
                        self.id.clone(),
                        node.clone(),
                        Body::new(
                            Some(self.next_msg_id()),
                            Payload::Gossip{messages: self.messages.clone()},
                        ),
                    );
                    msg.send(output)?;
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
