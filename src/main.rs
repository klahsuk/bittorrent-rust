use anyhow::Context;
use clap::{Parser, Subcommand};
use codecrafters_bittorrent::peer::{Handshake, MessageFramer, MessageTag, PeerMessage, Piece, Request};
use codecrafters_bittorrent::peers::url_encode;
use codecrafters_bittorrent::torrent::*;
use codecrafters_bittorrent::tracker::*;
use futures_util::{SinkExt, StreamExt};
use serde_bencode;
use serde_json::{Map, Value};
use sha1::Digest;
use std::net::SocketAddrV4;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

const BLOCK_MAX_SIZE: u64 = 1 << 14;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "snake_case")]
enum Command {
    Decode {
        value: String,
    },

    Info {
        torrent: PathBuf,
    },

    Peers {
        torrent: PathBuf,
    },

    Handshake {
        torrent: PathBuf,
        peer: String,
    },

    DownloadPiece {
        #[arg(short)]
        output: PathBuf,
        torrent: PathBuf,
        piece_id: usize,
    },
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str) {
    // Example: "5:hello" -> "hello"
    if let Some(start) = encoded_value.chars().next() {
        match start {
            'i' => parse_int(encoded_value),
            'l' => parse_list(encoded_value),
            'd' => parse_dict(encoded_value),
            '0'..='9' => parse_string(encoded_value),
            _ => panic!("Unhandled encoded value: {}", encoded_value),
        }
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

fn parse_int(encoded_value: &str) -> (Value, &str) {
    if let Some(end) = encoded_value.find('e') {
        let val = Value::Number(encoded_value[1..end].parse().unwrap());
        return (val, &encoded_value[end + 1..]);
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

fn parse_string(encoded_value: &str) -> (Value, &str) {
    if let Some((len, rest)) = encoded_value.split_once(":") {
        if let Ok(len) = len.parse::<usize>() {
            let val = Value::String(rest[..len].to_string());
            return (val, &rest[len..]);
        } else {
            panic!("Unhandled encoded value: {}:{}", len, rest)
        }
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

fn parse_list(encoded_value: &str) -> (Value, &str) {
    let mut lst = Vec::new();
    let mut rest = encoded_value.split_at(1).1;

    while !rest.is_empty() && !rest.starts_with('e') {
        let (val, remainder) = decode_bencoded_value(rest);

        lst.push(val);
        rest = remainder;
    }
    return (Value::Array(lst), &rest.split_at(1).1);
}

fn parse_dict(encoded_value: &str) -> (Value, &str) {
    let mut dict = Map::new();
    let mut rest = encoded_value.split_at(1).1;
    while !rest.is_empty() && !rest.starts_with('e') {
        let (key, val_slice) = parse_string(rest);
        let (v, remainder) = decode_bencoded_value(val_slice);
        rest = remainder;
        if let Value::String(k) = key {
            dict.insert(k, v);
        } else {
            panic!("keys need to be strings")
        }
    }
    return (Value::Object(dict), &rest.split_at(1).1);
}

fn load_torrent_file<T>(file_path: T) -> anyhow::Result<Torrent>
where
    T: Into<PathBuf>,
{
    let file = std::fs::read(file_path.into()).context("read torrent file")?;
    let torrent: Torrent = serde_bencode::from_bytes(&file).context("parse torrent file")?;
    Ok(torrent)
}

// Usage: your_program.sh decode "<encoded_value>"
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = Args::parse().command;

    match command {
        Command::Decode { value } => {
            let v = decode_bencoded_value(&value).0;
            println!("{v}");
            Ok(())
        }

        Command::Info { torrent } => {
            let t = load_torrent_file(torrent).unwrap();
            let info_hash = t.info_hash();
            println!("Tracker URL: {}", t.announce);
            if let Keys::SingleFile { length } = t.info.keys {
                println!("Length: {length}");
                println!("Info Hash: {}", hex::encode(info_hash));
                println!("Piece Length: {}", &t.info.piece_length);
                println!("Piece Hashes:");
                t.info
                    .pieces
                    .0
                    .iter()
                    .for_each(|h| println!("{}", hex::encode(h)));
                Ok(())
            } else {
                todo!();
            }
        }

        Command::Peers { torrent } => {
            let t = load_torrent_file(torrent).unwrap();
            let length = if let Keys::SingleFile { length } = t.info.keys {
                length
            } else {
                todo!()
            };
            let info_hash = t.info_hash();
            let request = TrackerRequest {
                peer_id: String::from("99887766554433221100"),
                port: 6861,
                uploaded: 0,
                downloaded: 0,
                left: length,
                compact: 1,
            };

            let url_params =
                serde_urlencoded::to_string(&request).context("url-encoding tracker params")?;
            let tracker_url = format!(
                "{}?{}&info_hash={}",
                t.announce,
                url_params,
                &url_encode(&info_hash)
            );

            let response = reqwest::get(tracker_url).await.context("fetch tracker")?;
            let response = response.bytes().await.context("fetch tracker response")?;
            eprintln!("{response:?}");
            let response: TrackerResponse =
                serde_bencode::from_bytes(&response).context("extracting Tracker Response")?;

            for peer in &response.peers.0 {
                println!("{}:{}", peer.ip(), peer.port());
            }

            Ok(())
        }

        Command::Handshake { torrent, peer } => {
            let t = load_torrent_file(torrent).unwrap();
            let info_hash = t.info_hash();
            let peer = peer.parse::<SocketAddrV4>().context("parsing the peer")?;
            let mut peer = tokio::net::TcpStream::connect(peer)
                .await
                .context("connect to peer")?;

            let mut handshake = Handshake::new(info_hash, b"99887766554433221100".clone());
            let handshake_bytes = Handshake::as_bytes_mut(&mut handshake);

            let _ = peer
                .write_all(handshake_bytes)
                .await
                .context("sending handshake");

            peer.read_exact(handshake_bytes)
                .await
                .context("reading response handshake")?;

            println!("Peer ID: {}", hex::encode(handshake.peer_id));

            Ok(())
        }

        Command::DownloadPiece {
            output,
            torrent,
            piece_id,
        } => {
            let t = load_torrent_file(torrent).unwrap();
            let peer_id = String::from("99887766554433221100");
            let length = if let Keys::SingleFile { length } = t.info.keys {
                length
            } else {
                todo!()
            };

            assert!(piece_id < t.info.pieces.0.len());
            let info_hash = t.info_hash();
            let request = TrackerRequest {
                peer_id: peer_id,
                port: 6861,
                uploaded: 0,
                downloaded: 0,
                left: length,
                compact: 1,
            };

            let url_params =
                serde_urlencoded::to_string(&request).context("url-encoding tracker params")?;
            let tracker_url = format!(
                "{}?{}&info_hash={}",
                t.announce,
                url_params,
                &url_encode(&info_hash)
            );

            let response = reqwest::get(tracker_url).await.context("fetch tracker")?;
            let response = response.bytes().await.context("fetch tracker response")?;
            eprintln!("{response:?}");
            let tracker_info: TrackerResponse =
                serde_bencode::from_bytes(&response).context("extracting Tracker Response")?;

            let peer = tracker_info.peers.0[0];
            let mut peer = tokio::net::TcpStream::connect(peer)
                .await
                .context("connect to peer")?;

            let mut handshake = Handshake::new(info_hash, b"99887766554433221100".clone());
            println!("Peer ID: {}", hex::encode(handshake.peer_id));

            let handshake_bytes =
                &mut handshake as *mut Handshake as *mut [u8; std::mem::size_of::<Handshake>()];

            let handshake_bytes: &mut [u8; std::mem::size_of::<Handshake>()] =
                unsafe { &mut *handshake_bytes };

            let _ = peer
                .write_all(handshake_bytes)
                .await
                .context("sending handshake");

            peer.read_exact(handshake_bytes)
                .await
                .context("reading response handshake")?;

            assert_eq!(handshake.length, 19);
            assert_eq!(&handshake.protocol, b"BitTorrent protocol");

            let mut peer = tokio_util::codec::Framed::new(peer, MessageFramer);

            let bitfield = peer
                .next()
                .await
                .expect("peer always sends bitfield")
                .context("peer message was invalid")?;

            assert_eq!(bitfield.tag, MessageTag::BitField);

            peer.send(PeerMessage {
                tag: MessageTag::Interested,
                payload: Vec::new(),
            })
            .await
            .context("sending interested message");

            let unchoke = peer
                .next()
                .await
                .expect("should be an unchoke")
                .context("waiting for unchoke")?;

            assert_eq!(unchoke.tag, MessageTag::Unchoke);
            assert!(unchoke.payload.is_empty());

            let piece_hash = t.info.pieces.0[piece_id];

            //use piece_length (16kb) to split the piece into
            // chunks send request for each except last one which needs to be calculated
            //then compare hash
            let piece_size = if piece_id == t.info.pieces.0.len() + 1 {
            let md = length % t.info.piece_length as usize;
                if md == 0 {
                    t.info.piece_length
                } else {
                    md as u64
                }
            } else {
                t.info.piece_length
            };

            //BLOCK_MAX_SIZE + 1 rounds up
            let num_blocks = (piece_size + (BLOCK_MAX_SIZE - 1)) / BLOCK_MAX_SIZE;
            let mut all_blocks = Vec::with_capacity(piece_size as usize);
            
            for b in 0..num_blocks {
                let block_size = if b == num_blocks -1 {
                    piece_size & BLOCK_MAX_SIZE
                } else {
                    BLOCK_MAX_SIZE
                };
                let mut request = Request::new(
                    piece_id as u32,
                    b as u32,
                    block_size as u32
                );

                let request_bytes = Vec::from(request.as_bytes_mut());

                peer.send(
                    PeerMessage { tag: MessageTag::Request, payload: request_bytes }
                ).await
                .with_context(|| format!("sending request for block {}", b))?;

             let block = peer
                .next()
                .await
                .expect("should be the block")
                .context("getting blocks of the piece")? ;

            assert_eq!(block.tag, MessageTag::Piece);
            assert!(!block.payload.is_empty());

                //response needs to be added into all blocks

            let piece = (&block.payload[..]) as *const [u8] as *const Piece;
            let piece = unsafe {
                &*piece
            };
            
             all_blocks.extend(piece.block());
            }

            let mut hasher = sha1::Sha1::new();
            hasher.update(&all_blocks);
            let hash: [u8; 20] = hasher
            .finalize()
            .try_into()
            .expect("Generic Array<_, 20> == [_; 20]");

            assert_eq!(all_blocks.len() as u64, piece_size);
            assert_eq!(hash, piece_hash);
            
            Ok(())
        }
    }
}
