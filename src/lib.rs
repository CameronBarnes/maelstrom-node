use std::io::StdoutLock;

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize, de::DeserializeOwned};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<Payload> {
    pub src: String,
    pub dest: String,
    pub body: Body<Payload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Payload> {
    #[serde(rename = "msg_id")]
    pub id: Option<usize>,
    #[serde(rename = "in_reply_to")]
    pub reply_to: Option<usize>,
    #[serde(flatten)]
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum InitPayload {
    Init{
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk{},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ErrorPayload {
    Error {
        code: usize,
        text: Option<String>,
    }
}

pub trait Node<Payload> {
    fn process_message(&mut self, msg: Message<Payload>, output: &mut StdoutLock) -> Result<()>;
}

pub fn main_loop<State, Payload>(mut state: State) -> Result<()> where State: Node<Payload>, Payload: DeserializeOwned {
    let stdin = std::io::stdin().lock();
    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message<Payload>>();

    let mut stdout = std::io::stdout().lock();

    for input in inputs {
        let input = input.context("serialize maelstrom input")?;
        state.process_message(input, &mut stdout)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    
}
