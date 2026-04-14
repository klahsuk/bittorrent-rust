use crate::peers::Peers;

use serde::{Serialize, Deserialize};

// Tracker GET request
// You'll need to make a request to the tracker URL you extracted in the previous stage, and include these query params

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrackerRequest {
    pub peer_id: String,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub left: usize,
    pub compact: u8
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrackerResponse {
    pub interval: usize,
    pub peers: Peers
}


