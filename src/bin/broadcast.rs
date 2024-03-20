use std::io::StdoutLock;

use anyhow::Result;
use maelstrom_node::{main_loop, Message, Node};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Payload {
    Broadcast { message: Value },
    BroadcastOk {},
    Read {},
    ReadOk { messages: Vec<Value> },
    Topology { topology: Value },
    TopologyOk {},
}

#[derive(Default)]
struct BroadcastNode {
    id: String,
    neighbours: Vec<String>,
    msg_id: usize,
    messages: Vec<Value>,
    callbacks: Vec<usize>,
}

impl BroadcastNode {
    fn broadcast(&mut self, msg: Message<Payload>, payload: Payload, output: &mut StdoutLock) -> Result<Vec<usize>> {

        let mut msg_ids_out = Vec::new();

        for id in self.neighbours.clone() {
            let msg = msg.clone();
            let mut msg = self.reply(msg, payload.clone());
            msg.dest = id.clone();
            msg_ids_out.push(msg.body.id.expect("Should always be present"));
            msg.send(output)?;
        }

        Ok(msg_ids_out)

    }
}

fn remove_if_present(item: usize, vec: &mut Vec<usize>) -> bool {
    for i in 0..vec.len() {
        if vec[i] == item {
            vec.swap_remove(i);
            return true;
        }
    }
    false
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
            Payload::Broadcast { message } => {
                dbg!(&message);
                if !msg.body.id.is_some_and(|id| remove_if_present(id, &mut self.callbacks)) {
                    self.messages.push(message.clone());
                    let mut send_ids = self.broadcast(msg.clone(), Payload::Broadcast{message}, &mut *output)?;
                    self.callbacks.append(&mut send_ids);
                }
                self.reply(msg, Payload::BroadcastOk {}).send(output)?;
            }
            Payload::Read {} => {
                self.reply(msg, Payload::ReadOk{ messages: self.messages.clone() }).send(output)?;
            },
            Payload::Topology { .. } => {
                self.reply(msg, Payload::TopologyOk{}).send(output)?;
            },
            _ => {}
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<BroadcastNode, Payload>()
}
