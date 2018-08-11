use nt::{EntryValue, EntryData, EntryType};

pub enum Message {
    Add(EntryData),
}

pub fn parse_message(line: String) -> Option<Message> {
    if line.starts_with("add ") {
        let args = &line[4..];
        let mut args = args.split(" ");
        info!("{:?}", args);
//        info!("{:?}", args.nth(0));
//        info!("{}", args.nth(0).unwrap());
//        let x = args.nth(0).unwrap();
        let ty = EntryType::from(args.next().expect("Failed to get args 0"));
//        let ty = EntryType::from(x);
        let value = match ty {
            EntryType::String => EntryValue::String(args.next().expect("Failed to get args 1").to_string()),
            EntryType::Boolean => EntryValue::Boolean(args.next().expect("Failed to get args 1 (bool)").parse().unwrap()),
            EntryType::Double => EntryValue::Double(args.next().expect("Failed to get args 1 (double)").parse().unwrap()),
            _ => unreachable!()
        };
        let name = args.next().expect("Failed to get args 2").trim().to_string();

        let dat = EntryData::new(name, 0u8, value);
        return Some(Message::Add(dat));
    }

    None
}