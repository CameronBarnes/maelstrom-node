use std::io::StdoutLock;

use anyhow::Result;
use maelstrom_node::{main_loop, Message, Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Payload {
    Broadcast {
        message: Value,
        callback: Option<String>,
    },
    BroadcastOk {},
    Read {},
    ReadOk {
        messages: Vec<Value>,
    },
    Topology {
        topology: Value,
    },
    TopologyOk {},
}

#[derive(Default)]
struct BroadcastNode {
    id: String,
    neighbours: Vec<String>,
    msg_id: usize,
    messages: Vec<Value>,
    callbacks: Vec<String>,
}

impl BroadcastNode {
    fn broadcast(
        &mut self,
        msg: Message<Payload>,
        payload: Payload,
        output: &mut StdoutLock,
    ) -> Result<()> {
        for id in self.neighbours.clone() {
            let msg = msg.clone();
            let mut msg = self.reply(msg, payload.clone());
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
        let neighbours = init.node_ids;
        //neighbours.retain_mut(|id| (*id).ne(&init.node_id));
        Ok(BroadcastNode {
            id: init.node_id,
            neighbours,
            msg_id: 1,
            messages: Vec::new(),
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
            Payload::Broadcast { message, callback } => {
                if !callback.is_some_and(|callback_id| self.callbacks.contains(&callback_id)) {
                    self.messages.push(message.clone());
                    let callback = generate_unique_id();
                    self.callbacks.push(callback.clone());
                    self.broadcast(
                        msg.clone(),
                        Payload::Broadcast {
                            message,
                            callback: Some(callback),
                        },
                        &mut *output,
                    )?;
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
            Payload::Topology { .. } => {
                self.reply(msg, Payload::TopologyOk {}).send(output)?;
            }
            _ => {}
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<BroadcastNode, Payload>()
}
