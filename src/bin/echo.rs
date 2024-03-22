use std::{io::StdoutLock, sync::mpsc::Sender};

use anyhow::Result;
use maelstrom_node::{main_loop, Node, Event};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Payload {
    Echo { echo: String },
    EchoOk { echo: String },
}

#[derive(Default)]
struct EchoNode {
    id: String,
    msg_id: usize,
}

impl Node<Payload, ()> for EchoNode {
    fn from_init(init: maelstrom_node::Init, _tx: Sender<Event<Payload, ()>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(EchoNode {
            id: init.node_id,
            msg_id: 1,
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

    fn process_message(&mut self, event: Event<Payload, ()>, output: &mut StdoutLock) -> Result<()> {
        let Event::Message(msg) = event else {
            panic!("Injected event where there's not supposed to be");
        };

        match msg.body.payload.clone() {
            Payload::Echo { echo } => {
                self.reply(msg, Payload::EchoOk { echo }).send(output)?;
            }
            Payload::EchoOk { .. } => {}
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<EchoNode, Payload, ()>()
}
