use crate::hashes::Hashes;
use serde::{Serialize, Deserialize};
use anyhow::{Context};
use sha1::Digest;


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Torrent {
    pub announce: String,
    pub info: Info
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Info {

    //suggested name to save the file / directory as
    pub name: String,
    #[serde(rename = "piece length")]

    //number of bytes in each piece, an integer
    pub piece_length: u64,

    //concatenated SHA-1 hashes of each piece (20 bytes each), a string
    pub pieces: Hashes,

    #[serde(flatten)]
    pub keys: Keys
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Keys{
    SingleFile{
        //size of the file in bytes, for single-file torrents
        length: usize,
    },

    MultiFile{
        files: Vec<File>
    }

}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct File {
    length: usize,
    path: Vec<String>
}

impl Torrent {
    pub fn info_hash(&self) -> [u8; 20] {
            let info_dict_encoded = serde_bencode::to_bytes(&self.info)
            .context("re-encoding the info dict").unwrap();
            let mut hasher = sha1::Sha1::new();
            hasher.update(&info_dict_encoded);
            let info_hash = hasher.finalize();
            info_hash.try_into().expect("")
    }
}
