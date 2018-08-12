use syn::{Ident, Type, PathSegment, Attribute, Meta, Lit};

pub fn parse_type(ty: &Type) -> Option<Ident> {
    match ty {
        &Type::Path(ref path) => {
            let segment_name: &PathSegment = path.path.segments.iter().last().unwrap();
            Some(segment_name.ident.clone())
        }

        _ => None
    }
}

pub fn parse_packet_id(attrs: Vec<Attribute>) -> Option<u8> {
    for attr in attrs {
        if let Some(meta) = attr.interpret_meta() {
            match meta {
                Meta::NameValue(named_value) => {
                    if named_value.ident.to_string() == "packet_id".to_string() {
                        match named_value.lit {
                            Lit::Int(i) => return Some(i.value() as u8),
                            _ => return None
                        }
                    }
                }
                _ => return None
            }
        }
    }

    None
}