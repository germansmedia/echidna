// Echidna - Data

use {
    tokio::{
        io,
        io::AsyncWriteExt,
    },
    codec::Codec,
    std::{
        collections::HashMap,
        net::SocketAddr,
    },
};

pub const CHUNK_SIZE: usize = 16384;

pub const CHUNKS_PER_HEARTBEAT: usize = 16;

pub const CHUNKS_PER_ACK: usize = 8;

pub const CHUNKS_PER_NACK: usize = 8;

pub const RETRANSMIT_DELAY_USEC: usize = 100;

pub const CHUNKS_PER_INITIAL_BURST: usize = 8;

pub type MessageId = u64;
pub type ParticipantId = u64;
pub type PublisherId = u64;
pub type SubscriberId = u64;

#[derive(Codec)]
pub struct Chunk {
    pub ts: u64,
    pub id: MessageId,
    pub total_bytes: u64,
    pub total: u32,
    pub index: u32,
    pub data: Vec<u8>,
}

#[derive(Codec)]
pub enum PublisherToSubscriber {
    Heartbeat(MessageId),
    Chunk(Chunk),
}

#[derive(Codec)]
pub enum SubscriberToPublisher {
    Ack(SubscriberId,MessageId,Vec<u32>),
    NAck(SubscriberId,MessageId,Vec<u32>),
}

#[derive(Codec)]
pub struct Beacon {
    pub id: ParticipantId,
    pub port: u16,
}

#[derive(Clone,Codec)]
pub struct PublisherRef {
    pub topic: String,
}

#[derive(Clone,Codec)]
pub struct SubscriberRef {
    pub address: SocketAddr,
    pub topic: String,
}

#[derive(Codec)]
pub struct ParticipantAnnounce {
    pub id: ParticipantId,
    pub pubs: HashMap<PublisherId,PublisherRef>,
    pub subs: HashMap<SubscriberId,SubscriberRef>,
}

#[derive(Codec)]
pub enum ParticipantToParticipant {
    NewPub(PublisherId,PublisherRef),
    DropPub(PublisherId),
    NewSub(SubscriberId,SubscriberRef),
    DropSub(SubscriberId),
}

#[derive(Codec)]
pub enum ToParticipant {
    InitPub(PublisherId,PublisherRef),
    InitSub(SubscriberId,SubscriberRef),
}

#[derive(Codec)]
pub enum ParticipantToPublisher {
    Init(HashMap<SubscriberId,SubscriberRef>),
    InitFailed,
    NewSub(SubscriberId,SubscriberRef),
    DropSub(SubscriberId),
}

#[derive(Codec)]
pub enum ParticipantToSubscriber {
    Init,
    InitFailed,
}

pub async fn send_message<S: io::AsyncWrite + Unpin,M: Codec>(stream: &mut S,message: M) {
    let mut send_buffer = Vec::new();
    message.encode(&mut send_buffer);
    stream.write_all(&send_buffer).await.expect("cannot send message");
}
