// Echidna - Data

use {
    tokio::{
        io,
        io::AsyncWriteExt,
    },
    codec::Codec,
    std::{
        collections::HashMap,
    },
};

pub type PeerId = u64;
pub type PubId = u64;
pub type SubId = u64;

#[derive(Codec)]
pub struct Beacon {
    pub id: PeerId,
    pub port: u16,
}

#[derive(Clone,Codec)]
pub struct PubRef {
    pub topic: String,
}

#[derive(Clone,Codec)]
pub struct SubRef {
    pub port: u16,
    pub topic: String,
}

#[derive(Codec)]
pub struct PeerAnnounce {
    pub id: PeerId,
    pub pubs: HashMap<PubId,PubRef>,
    pub subs: HashMap<SubId,SubRef>,
}

#[derive(Codec)]
pub enum PeerToPeer {
    NewPub(PubId,PubRef),
    DropPub(PubId),
    NewSub(SubId,SubRef),
    DropSub(SubId),
}

#[derive(Codec)]
pub enum ToPart {
    InitPub(PubId,PubRef),
    InitSub(SubId,SubRef),
}

#[derive(Codec)]
pub enum PartToPub {
    Init(HashMap<SubId,SubRef>),
    InitFailed,
    NewSub(SubId,SubRef),
    DropSub(SubId),
}

#[derive(Codec)]
pub enum PartToSub {
    Init,
    InitFailed,
}

pub async fn send_message<S: io::AsyncWrite + Unpin,M: Codec>(stream: &mut S,message: M) {
    let mut send_buffer = Vec::new();
    message.encode(&mut send_buffer);
    stream.write_all(&send_buffer).await.expect("cannot send message");
}