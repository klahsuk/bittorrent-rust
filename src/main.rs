use std::env;

// Available if you need it!
// use serde_bencode

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    // If encoded_value starts with a digit, it's a number
        // Example: "5:hello" -> "hello"
        if encoded_value.chars().nth(0) == Some('i'){
            if let Some(end) = encoded_value.find('e'){
                return serde_json::Value::String(encoded_value[1..end].to_string())
            } else {
                panic!("Unhandled encoded value: {}", encoded_value)
            }

        }
        if let Some((len, rest)) = encoded_value.split_once(":"){
            if let Ok(len) = len.parse::<usize>(){
                return serde_json::Value::String(rest[..len].to_string());
            }
            else {
                panic!("Unhandled encoded value: {}", encoded_value)
            }
        } else {
            panic!("Unhandled encoded value: {}", encoded_value)
        }   
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
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
