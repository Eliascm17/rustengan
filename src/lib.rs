use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::StdoutLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<Payload> {
    /// A string identifying the node this message came from
    pub src: String,
    /// A string identifying the node this message is to
    #[serde(rename = "dest")]
    pub dst: String,
    /// An object: the payload of the message
    pub body: Body<Payload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Payload> {
    /// (optional)  A unique integer identifier
    #[serde(rename = "msg_id")]
    pub id: Option<usize>,
    /// (optional)  For req/response, the msg_id of the request
    pub in_reply_to: Option<usize>,
    /// (mandatory) A string identifying the type of message this is
    #[serde(flatten)]
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

pub trait Node<Payload> {
    fn step(&mut self, input: Message<Payload>, output: &mut StdoutLock) -> anyhow::Result<()>;
}

pub fn main_loop<S, Payload>(mut state: S) -> anyhow::Result<()>
where
    S: Node<Payload>,
    Payload: DeserializeOwned,
{
    // define stdin and define stream from stdin that expects messages from other nodes
    let stdin = std::io::stdin().lock();
    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message<Payload>>();

    let mut stdout = std::io::stdout().lock();

    // for all of the messages coming through stdin
    for input in inputs {
        // deserialize input from input stream into `Message`
        let input =
            input.context("Maelstrom input from STDIN into `Message` could not be deserialized")?;

        // handle input message
        state
            .step(input, &mut stdout)
            .context("Node steph function failed")?;
    }

    Ok(())
}
