use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::{BufRead, StdoutLock, Write};

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
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum InitPayload {
    /// At the start of a test, Maelstrom issues a single init message to each node
    Init(Init),
    /// In response to the init message, each node must respond with a message of type init_ok
    InitOk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

pub trait Node<S, Payload> {
    fn from_init(state: S, init: Init) -> anyhow::Result<Self>
    where
        Self: Sized;
    fn step(&mut self, input: Message<Payload>, output: &mut StdoutLock) -> anyhow::Result<()>;
}

pub fn main_loop<S, N, P>(init_state: S) -> anyhow::Result<()>
where
    P: DeserializeOwned,
    N: Node<S, P>,
{
    let stdin = std::io::stdin().lock();
    let mut stdin = stdin.lines();
    let mut stdout = std::io::stdout().lock();

    let init_msg: Message<InitPayload> = serde_json::from_str(
        &stdin
            .next()
            .expect("no init message received")
            .context("failed to read init message from stdin")?,
    )
    .context("init message could not be deserialized")?;
    let InitPayload::Init(init) = init_msg.body.payload else {
        panic!("first message should be init")
    };

    let mut node: N = Node::from_init(init_state, init).context("node initialization failed")?;

    let reply = Message {
        src: init_msg.dst,
        dst: init_msg.src,
        body: Body {
            id: Some(0),
            in_reply_to: init_msg.body.id,
            payload: InitPayload::InitOk,
        },
    };
    serde_json::to_writer(&mut stdout, &reply).context("serialize response to init")?;
    stdout.write_all(b"\n").context("write trailing newline")?;

    for line in stdin {
        // deserialize input from input stream into `Message`
        let line =
            line.context("Maelstrom input from STDIN into `Message` could not be deserialized")?;
        let input = serde_json::from_str(&line)
            .context("Maelstrom input from STDIN could not be deserialized")?;

        // handle input message
        node.step(input, &mut stdout)
            .context("Node steph function failed")?;
    }

    Ok(())
}
