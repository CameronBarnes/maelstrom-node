use std::io::{BufRead, StdoutLock, Write};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<Payload> {
    pub src: String,
    pub dest: String,
    pub body: Body<Payload>,
}

impl<Payload> Message<Payload> {
    pub fn send(&self, output: &mut impl Write) -> Result<()>
    where
        Payload: Serialize,
    {
        serde_json::to_writer(&mut *output, self).context("serialize response message")?;
        output.write_all(b"\n").context("write trailing newline")?;
        Ok(())
    }
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
enum InitPayload {
    Init(Init),
    InitOk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

pub trait Node<Payload> {
    fn from_init(init: Init) -> Result<Self>
    where
        Self: Sized;
    fn next_msg_id(&mut self) -> usize;
    fn node_id(&self) -> String;
    fn process_message(&mut self, msg: Message<Payload>, output: &mut StdoutLock) -> Result<()>;
    fn reply(&mut self, msg: Message<Payload>, payload: Payload) -> Message<Payload> {
        let mut body = msg.body;
        body.reply_to = body.id;
        body.id = Some(self.next_msg_id());
        body.payload = payload;
        Message {
            src: self.node_id(),
            dest: msg.src,
            body,
        }
    }
}

pub fn main_loop<State, Payload>() -> Result<()>
where
    State: Node<Payload>,
    Payload: DeserializeOwned,
{
    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();
    let mut stdout = std::io::stdout().lock();

    let init_msg: Message<InitPayload> = serde_json::from_str(
        &stdin
            .next()
            .expect("should receive an init message")
            .context("failed to read init msg from stdin")?,
    )
    .context("init message could not be deserialized")?;
    let InitPayload::Init(init) = init_msg.body.payload else {
        panic!("first message should be init");
    };

    Message {
        src: init.node_id.clone(),
        dest: init_msg.src.clone(),
        body: Body {
            id: Some(0),
            reply_to: init_msg.body.id,
            payload: InitPayload::InitOk,
        },
    }
    .send(&mut stdout)?;

    let mut state: State = Node::from_init(init).context("node init failed")?;

    for line in stdin {
        let input = line.context("get stdin")?;
        let input = serde_json::from_str(&input).context("serialize input")?;
        state.process_message(input, &mut stdout)?;
    }

    Ok(())
}
