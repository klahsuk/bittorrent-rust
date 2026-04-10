use std::path::PathBuf;
use std::{env};
use serde_json::{Map, Value};
use serde::Deserialize;
use serde::Serialize;
use serde_bencode;

// Available if you need it!
// use serde_bencode

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Torrent {
    announce: String,
    info: Info
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Info {
    length: usize,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: u64,
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>
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
    let file = std::fs::read(file_path.into())?;
    let torrent: Torrent = serde_bencode::from_bytes(&file)?;
    Ok(torrent)
}

// Usage: your_program.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        // TODO: Uncomment the code below to pass the first stage
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.0.to_string());
    }   else if command == "info" {
        let file_path = &args[2];
        let torrent = load_torrent_file(file_path).expect("something was wrong with the file path");
        println!("Tracker URL: {}", torrent.announce);
        println!("File Length: {}", torrent.info.length)
    } 
    else {
        println!("unknown command: {}", args[1])
    }
}
