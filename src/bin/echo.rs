use std::io::Write;

use anyhow::{Context, Result};
use maelstrom_node::{Body, Message, Payload::*};

struct EchoNode {
    id: Option<String>,
    msg_id: usize,
}

impl EchoNode {
    pub fn process_message(&mut self, msg: Message) -> Result<Vec<Message>> {
        let mut output = Vec::new();

        match msg.body.payload {
            Init { node_id, .. } => {
                self.id = Some(node_id.clone());
                let body = Body {
                    id: Some(self.msg_id),
                    reply_to: msg.body.id,
                    payload: InitOk {},
                };
                self.msg_id += 1;
                output.push(Message {
                    src: node_id.clone(),
                    dest: msg.src,
                    body,
                });
            }
            Echo { echo } => {
                if let Some(id) = &self.id {
                    let body = Body {
                        id: Some(self.msg_id),
                        reply_to: msg.body.id,
                        payload: EchoOk { echo },
                    };
                    self.msg_id += 1;
                    output.push(Message {
                        src: id.clone(),
                        dest: msg.src,
                        body,
                    });
                } else {
                    let body = Body {
                        id: Some(self.msg_id),
                        reply_to: msg.body.id,
                        payload: Error {
                            code: 11,
                            text: Some(String::from("Have not yet receieved Init")),
                        },
                    };
                    self.msg_id += 1;
                    output.push(Message {
                        src: String::from("uninitialized"),
                        dest: msg.src,
                        body,
                    });
                }
            }
            EchoOk { .. } => {}
            _ => {
                if let Some(id) = &self.id {
                    let body = Body {
                        id: Some(self.msg_id),
                        reply_to: msg.body.id,
                        payload: Error {
                            code: 10,
                            text: Some(String::from("unimplemented or not supported")),
                        },
                    };
                    self.msg_id += 1;
                    output.push(Message {
                        src: id.clone(),
                        dest: msg.src,
                        body,
                    });
                } else {
                    let body = Body {
                        id: Some(self.msg_id),
                        reply_to: msg.body.id,
                        payload: Error {
                            code: 11,
                            text: Some(String::from("Have not yet receieved Init")),
                        },
                    };
                    self.msg_id += 1;
                    output.push(Message {
                        src: String::from("uninitialized"),
                        dest: msg.src,
                        body,
                    });
                }
            }
        }

        Ok(output)
    }
}

pub fn main() -> Result<()> {
    let stdin = std::io::stdin().lock();
    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message>();

    let mut stdout = std::io::stdout().lock();

    let mut state = EchoNode {
        id: None,
        msg_id: 0,
    };

    for input in inputs {
        let input = input.context("serialize maelstrom input")?;
        let out = state.process_message(input)?;
        for msg in out {
            serde_json::to_writer(&mut stdout, &msg).context("serialize response")?;
            stdout.write_all(b"\n").context("write trailing newline")?;
        }
    }

    Ok(())
}
