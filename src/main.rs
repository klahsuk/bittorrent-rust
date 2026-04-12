use core::hash;
use std::path::PathBuf;
use serde_json::{Map, Value};
use serde_bencode;
use hashes::Hashes;
use serde::{Deserialize, Serialize};
use clap::{Parser, Subcommand};
use anyhow::Context;
use sha1::Digest;

// Available if you need it!
// use serde_bencode


mod hashes {
    use serde::Serializer;
    use serde::{Serialize, de::{self, Visitor}};
    use std::{fmt};
    use serde::Deserialize;
    struct HashesVisitor;
    
    #[derive(Debug, Clone)]
    pub struct Hashes(pub Vec<[u8; 20]>);
    
    
    impl<'de> Visitor<'de> for HashesVisitor {
        type Value = Hashes;
    
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a byte string with lebgth a multiple of 20")
        }
    
        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.len() % 20 != 0{
                return 
                Err(E::custom(format!("vector len must be a multiple of 20: {} does not fit the criteria", v.len())));
            }
    
            Ok(Hashes(
                v.chunks_exact(20)
                .map(|slice_20| slice_20.try_into().unwrap())
                .collect()
            ))
        }
    }
    
    impl <'de> Deserialize<'de> for Hashes {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de> {
            deserializer.deserialize_bytes(HashesVisitor)
        }
    }

   impl Serialize for Hashes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let single_slice = self.0.concat();
        serializer.serialize_bytes(&single_slice)
    }
}

}


#[derive(Debug, Clone, Deserialize, Serialize)]
struct Torrent {
    announce: String,
    info: Info
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Info {
    
    //suggested name to save the file / directory as
    name: String,
    #[serde(rename = "piece length")]

    //number of bytes in each piece, an integer
    piece_length: u64,

    //concatenated SHA-1 hashes of each piece (20 bytes each), a string
    pieces: Hashes,

    #[serde(flatten)]
    keys: Keys
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum Keys{
    SingleFile{
    //size of the file in bytes, for single-file torrents
        length: usize,
    },

    MultiFile{
        files: Vec<File>
    }

}

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
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct File {
    length: usize,
    path: Vec<String>
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
fn main() -> anyhow::Result<()>{
    let args = Args::parse();

    match args.command {
        Command::Decode { value } => {
            let v = decode_bencoded_value(&value).0;
            println!("{v}");
        }
        Command::Info { torrent } => {
            let dot_torrent = std::fs::read(torrent).context("read torrent file")?;
            let t: Torrent =
                serde_bencode::from_bytes(&dot_torrent).context("parse torrent file")?;
            let info_dict_encoded = serde_bencode::to_bytes(&t.info).context("re-encoding the info dict")?;
            let mut hasher = sha1::Sha1::new();
            hasher.update(&info_dict_encoded);
            let info_hash = hasher.finalize();
            eprintln!("{t:?}");
            println!("Tracker URL: {}", t.announce);
            if let Keys::SingleFile { length } = t.info.keys {
                println!("Length: {length}");
                println!("Info Hash: {}", hex::encode(info_hash));
                println!("Piece Length: {}", &t.info.piece_length);
                println!("Piece Hashes : ");
                t.info.pieces.0.iter().for_each(|h| println!("{}", hex::encode(h)));
            } else {
                todo!();
            }
        }
    }

    Ok(())
}
