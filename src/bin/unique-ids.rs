use std::{io::StdoutLock, sync::mpsc::Sender};

use anyhow::Result;
use maelstrom_node::{main_loop, Node, Event};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Payload {
    Generate {},
    GenerateOk {
        #[serde(rename = "id")]
        guid: String,
    },
}

fn generate_unique_id() -> String {
    ulid::Ulid::new().to_string()
}

#[derive(Default)]
struct UniqueIdNode {
    id: String,
    msg_id: usize,
}

impl Node<Payload, ()> for UniqueIdNode {
    fn from_init(init: maelstrom_node::Init, _tx: Sender<Event<Payload, ()>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(UniqueIdNode {
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

        match msg.body.payload {
            Payload::Generate {} => {
                self.reply(
                    msg,
                    Payload::GenerateOk {
                        guid: generate_unique_id(),
                    },
                )
                .send(output)?;
            }
            Payload::GenerateOk { .. } => {}
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop::<UniqueIdNode, Payload, ()>()
}
