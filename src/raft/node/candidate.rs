use log::{warn, info, debug};
use rand::Rng;

use crate::raft::message::{Address, Event, Message, Response};
use crate::error::{Result, Error};

use super::{ELECTION_TIMEOUT_MIN, ELECTION_TIMEOUT_MAX, RoleNode, leader::{Leader}, follower::Follower, Node,};

pub struct Candidate {
    election_ticks: u64,
    election_timeout: u64,
    votes: u64,
}

impl Candidate {
    pub fn new() -> Self {
        Self {
            election_ticks: 0,
            election_timeout: rand::thread_rng()
                .gen_range(ELECTION_TIMEOUT_MIN..=ELECTION_TIMEOUT_MAX),
            votes: 1,
        }
    }
}

impl RoleNode<Candidate> {
    fn become_follower(mut self, term: u64, leader: &str) -> Result<RoleNode<Follower>> {
        info!("Discovered leader {} for term {}, following", leader, term);
        self.term = term;
        self.log.save_term(term, None)?;
        let mut node = 
            self.become_role(Follower::new(Some(leader), None))?;
        node.abort_proxied()?;
        node.forward_queued(Address::Peer(leader.to_string()))?;
        Ok(node)
    }

    fn become_leader(self) -> Result<RoleNode<Leader>> {
        info!("Won election for term {}, becoming leader", self.term);
        let peers = self.peers.clone();
        let last_index = self.log.last_index;
        let mut node = self.become_role(Leader::new(peers, last_index))?;
        node.send(
            Address::Peers,
            Event::Heartbeat { 
                commit_index: node.log.commited_index,
                commit_term: node.log.commited_term,
            },
        )?;
        node.append(None)?;
        node.abort_proxied()?;
        Ok(node)
    }

    pub fn step(mut self, msg: Message) -> Result<Node> {
        if let Err(err) = self.validate(&msg) {
            warn!("Ignoring invalid message: {}", err);
            return Ok(self.into());
        }
        if msg.term > self.term {
            if let Address::Peer(from) = &msg.from {
                return self.become_follower(msg.term, from)?.step(msg);
            }
        }

        match msg.event {
            Event::Heartbeat { .. } => {
                if let Address::Peer(from) = &msg.from {
                    return self.become_follower(msg.term, from)?.step(msg);
                }
            }

            Event::GrantVote => {
                debug!("Received term {} vote from {:?}", self.term, msg.from);
                self.role.votes += 1;
                if self.role.votes >= self.quorum() {
                    let queued = std::mem::take(&mut self.queued_reqs);
                    let mut node: Node = self.become_leader()?.into();
                    for (from, event) in queued {
                        node = node.step(Message {
                            from,
                            to: Address::Local,
                            term: 0,
                            event,
                        })?;
                    }
                    return Ok(node);
                }
            }
            Event::ClientRequest { .. } => self.queued_reqs.push((msg.from, msg.event)),
            Event::ClientResponse { id, mut response } => {
                if let Ok(Response::Status(ref mut status)) = response {
                    status.server = self.id.clone();
                }
                self.proxied_reqs.remove(&id);
                self.send(Address::Client, Event::ClientResponse { id, response, })?;
            }

            Event::SolicitVote { .. } => {}

            Event::ConfirmLeader { .. }
            | Event::ReplicateEntries { .. }
            | Event::AcceptEntries { .. }
            | Event::RejectEntries { .. } => warn!("Received unexpected message {:?}", msg),

        };

        Ok(self.into())
    }

    pub fn tick(mut self) -> Result<Node> {
        self.role.election_ticks += 1;
        if self.role.election_ticks >= self.role.election_timeout {
            info!("Election timed out, starting new election for term {}", self.term + 1);
            self.term += 1;
            self.log.save_term(self.term, None)?;
            self.role = Candidate::new();
            self.send(
                Address::Peers,
                Event::SolicitVote {
                    last_index: self.log.last_index,
                    last_term: self.log.last_term,
                },
            )?;
        }
        Ok(self.into())
    }
}