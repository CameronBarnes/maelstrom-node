use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub src: String,
    pub dest: String,
    pub body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
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
pub enum Payload {
    Init{
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk{},
    Echo{
        echo: String,
    },
    EchoOk {
        echo: String,
    },
    Error {
        code: usize,
        text: Option<String>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    
}
