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