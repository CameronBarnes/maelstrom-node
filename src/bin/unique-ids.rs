use std::io::{StdoutLock, Write};

use anyhow::{Result, Context};
use maelstrom_node::{Body, Message, Node, main_loop};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Payload {
    Init{
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk{},
    Generate{},
    GenerateOk{
        #[serde(rename = "id")]
        guid: String,
    },
    Error {
        code: usize,
        text: Option<String>,
    }
}

fn generate_unique_id() -> String {
    ulid::Ulid::new().to_string()
}

#[derive(Default)]
struct UniqueIdNode {
    id: Option<String>,
    msg_id: usize,
}

impl Node<Payload> for UniqueIdNode {
    fn process_message(&mut self, msg: Message<Payload>, output: &mut StdoutLock) -> Result<()> {
        match msg.body.payload {
            Payload::Init { node_id, .. } => {
                self.id = Some(node_id.clone());
                let body = Body {
                    id: Some(self.msg_id),
                    reply_to: msg.body.id,
                    payload: Payload::InitOk {},
                };
                self.msg_id += 1;
                serde_json::to_writer(&mut *output, &Message {
                    src: node_id.clone(),
                    dest: msg.src,
                    body,
                })?;
                output.write_all(b"\n").context("write trailing newline")?;
            }
            Payload::Generate {  } => {
                if let Some(id) = &self.id {
                    let body = Body {
                        id: Some(self.msg_id),
                        reply_to: msg.body.id,
                        payload: Payload::GenerateOk { guid: generate_unique_id() },
                    };
                    self.msg_id += 1;
                    serde_json::to_writer(&mut *output, &Message {
                        src: id.clone(),
                        dest: msg.src,
                        body,
                    })?;
                    output.write_all(b"\n").context("write trailing newline")?;
                } else {
                    let body = Body {
                        id: Some(self.msg_id),
                        reply_to: msg.body.id,
                        payload: Payload::Error {
                            code: 11,
                            text: Some(String::from("Have not yet receieved Init")),
                        },
                    };
                    self.msg_id += 1;
                    serde_json::to_writer(&mut *output, &Message {
                        src: String::from("uninitialized"),
                        dest: msg.src,
                        body,
                    })?;
                    output.write_all(b"\n").context("write trailing newline")?;
                }
            }
            Payload::GenerateOk { .. } => {}
            _ => {
                if let Some(id) = &self.id {
                    let body = Body {
                        id: Some(self.msg_id),
                        reply_to: msg.body.id,
                        payload: Payload::Error {
                            code: 10,
                            text: Some(String::from("unimplemented or not supported")),
                        },
                    };
                    self.msg_id += 1;
                    serde_json::to_writer(&mut *output, &Message {
                        src: id.clone(),
                        dest: msg.src,
                        body,
                    })?;
                    output.write_all(b"\n").context("write trailing newline")?;
                } else {
                    let body = Body {
                        id: Some(self.msg_id),
                        reply_to: msg.body.id,
                        payload: Payload::Error {
                            code: 11,
                            text: Some(String::from("Have not yet receieved Init")),
                        },
                    };
                    self.msg_id += 1;
                    serde_json::to_writer(&mut *output, &Message {
                        src: String::from("uninitialized"),
                        dest: msg.src,
                        body,
                    })?;
                    output.write_all(b"\n").context("write trailing newline")?;
                }
            }
        }

        Ok(())
    }
}

pub fn main() -> Result<()> {
    main_loop(UniqueIdNode::default())
}
