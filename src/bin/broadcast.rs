use std::{io::StdoutLock, collections::HashMap};

use anyhow::Result;
use maelstrom_node::{main_loop, Message, Node};
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

#[derive(Default)]
struct BroadcastNode {
    id: String,
    neighbours: Vec<String>,
    msg_id: usize,
    messages: Vec<usize>,
    seen_uids: Vec<String>,
    callbacks: Vec<(usize, String, usize, String)>,
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
            let mut msg = self.reply(msg, Payload::Broadcast{message, callback: callback.clone()});
            self.callbacks.push((message, id.clone(), msg.body.id.unwrap(), callback.id().clone()));
            msg.dest = id.clone();
            msg.send(output)?;
        }

        Ok(())
    }
}

fn generate_unique_id() -> String {
    ulid::Ulid::new().to_string()
}

impl Node<Payload> for BroadcastNode {
    fn from_init(init: maelstrom_node::Init) -> Result<Self>
    where
        Self: Sized,
    {
        let mut neighbours = init.node_ids;
        neighbours.retain_mut(|id| (*id).ne(&init.node_id));
        Ok(Self {
            id: init.node_id,
            neighbours,
            msg_id: 1,
            messages: Vec::new(),
            seen_uids: Vec::new(),
            callbacks: Vec::new(),
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

    fn process_message(&mut self, msg: Message<Payload>, output: &mut StdoutLock) -> Result<()> {
        match msg.body.payload.clone() {
            Payload::Broadcast { message, mut callback } => {
                if !self.seen_uids.contains(callback.id()) {
                    callback.nodes.push(self.id.clone());
                    self.messages.push(message);
                    self.seen_uids.push(callback.id().clone());
                    self.broadcast(
                        msg.clone(),
                        message,
                        callback.clone(),
                        &mut *output,
                    )?;
                }
                callback.nodes = vec![self.id.clone()];
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
            Payload::BroadcastOk { } => {
                self.callbacks.retain(|(_msg, node, msg_id, _uid)| {
                    msg.src.ne(node) && !msg.body.reply_to.is_some_and(|reply_id| reply_id == *msg_id)
                })
            }
            Payload::TopologyOk { .. } | Payload::ReadOk { .. } => {}
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<BroadcastNode, Payload>()
}
