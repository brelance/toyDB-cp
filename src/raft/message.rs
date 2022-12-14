use serde_derive::{Deserialize, Serialize};
use super::{log::Entry, Status};
use crate::error::{Result, Error};


#[derive(Clone, Debug,Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum Address {
    Peers,
    Peer(String),
    Local,
    Client,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct  Message {
    pub term: u64,
    pub from: Address,
    pub to: Address,
    pub event: Event,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    Heartbeat {
        commit_index: u64,
        commit_term: u64,
    },

    ConfirmLeader {
        commit_index: u64,
        has_commited: bool,
    },

    GrantVote,

    ReplicateEntries {
        base_index: u64,
        base_term: u64,
        entries: Vec<Entry>
    },

    AcceptEntries {
        last_index: u64,
    },

    RejectEntries,

    ClientRequest {
        id: Vec<u8>,
        request: Request,
    },

    ClientResponse {
        id: Vec<u8>,
        response: Result<Response>,
    },

    SolicitVote {
        last_index: u64,
        last_term: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Query(Vec<u8>),
    Mutate(Vec<u8>),
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    State(Vec<u8>),
    Status(Status),
}
