use std::path::PathBuf;
use anyhow::{Context};
use codecrafters_bittorrent::peers::url_encode;
use serde_json::{Map, Value};
use serde_bencode;
use clap::{Parser, Subcommand};
use codecrafters_bittorrent::torrent::*;
use codecrafters_bittorrent::tracker::*;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand, Debug)]
enum Command{
    Decode {
        value: String
    },

    Info {
        torrent: PathBuf
    },

    Peers {
        torrent: PathBuf
    }
}



#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (serde_json::Value, &str){
        // Example: "5:hello" -> "hello"
       if let Some (start) = encoded_value.chars().next(){
        match start {
            'i' => parse_int(encoded_value),
            'l' => parse_list(encoded_value),
            'd' => parse_dict(encoded_value),
            '0'..='9' => parse_string(encoded_value),
            _ => panic!("Unhandled encoded value: {}", encoded_value)
        }
       } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

fn parse_int(encoded_value: &str) -> (Value, &str) {
     if let Some(end) = encoded_value.find('e'){
        let val = Value::Number(encoded_value[1..end].parse().unwrap());
        return (val , &encoded_value[end + 1 ..])
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

fn parse_string(encoded_value: &str) -> (Value, &str) {
    if let Some((len, rest)) = encoded_value.split_once(":"){
        if let Ok(len) = len.parse::<usize>(){
            let val = Value::String(rest[..len].to_string());
            return (val , &rest[len..])
        }
        else {
            panic!("Unhandled encoded value: {}:{}",len, rest )
        }
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }   
}

fn parse_list(encoded_value: &str) -> (Value, &str){
    let mut lst = Vec::new();
    let mut rest = encoded_value.split_at(1).1;

    while !rest.is_empty() && !rest.starts_with('e'){
        let (val, remainder) = decode_bencoded_value(rest);

        lst.push(val);
        rest = remainder;
    }
    return (Value::Array(lst), &rest.split_at(1).1)
}

fn parse_dict(encoded_value: &str) -> (Value, &str){
    let mut dict = Map::new();
    let mut rest = encoded_value.split_at(1).1;
    while !rest.is_empty() && !rest.starts_with('e'){
       let (key, val_slice) = parse_string(rest);
       let (v, remainder) = decode_bencoded_value(val_slice);
       rest = remainder;
       if let Value::String(k) = key{
           dict.insert(k, v);
       } else {
           panic!("keys need to be strings")
       }
    }
    return (Value::Object(dict), &rest.split_at(1).1)
}

fn load_torrent_file<T>(file_path: T) -> anyhow::Result<Torrent> 
where T: Into<PathBuf> {
    let file = std::fs::read(file_path.into()).context("read torrent file")?;
    let torrent: Torrent = serde_bencode::from_bytes(&file).context("parse torrent file")?;
    Ok(torrent)
}

// Usage: your_program.sh decode "<encoded_value>"
#[tokio::main]
async fn main() -> anyhow::Result<()>{
    let command = Args::parse().command;

    match command {
        Command::Decode { value } => {
            let v = decode_bencoded_value(&value).0;
            println!("{v}");
            Ok(())
        },

        Command::Info { torrent } => {
            let t = load_torrent_file(torrent).unwrap();
            let info_hash = t.info_hash();
            println!("Tracker URL: {}", t.announce);
            if let Keys::SingleFile { length } = t.info.keys {
                println!("Length: {length}");
                println!("Info Hash: {}", hex::encode(info_hash));
                println!("Piece Length: {}", &t.info.piece_length);
                println!("Piece Hashes:");
                t.info.pieces.0.iter().for_each(|h| println!("{}", hex::encode(h)));
                Ok(())
            } else {
                todo!();
            }
        },

        Command:: Peers {torrent} => {
            let t = load_torrent_file(torrent).unwrap();
            let length = if let Keys::SingleFile { length } = t.info.keys { length }
                else {todo!()};
            let info_hash = t.info_hash();
            let request = TrackerRequest {
                peer_id: String::from("00112233445566778899"),
                port: 6861,
                uploaded: 0,
                downloaded:0,
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
            let response = response.bytes().await.context("parse tracker response")?;
            eprintln!("{response:?}");
            let response: TrackerResponse = 
                serde_bencode::from_bytes(&response).context("extracting Tracker Response")?;
            
            for peer in &response.peers.0 {
                println!("{}:{}", peer.ip(), peer.port());
            }
               
            Ok(())
        } 
    }

}
