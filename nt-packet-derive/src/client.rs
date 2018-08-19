
use super::util;
use proc_macro2::{TokenStream, Span};

use syn::{DeriveInput, Data, Fields, Ident};

pub fn gen_client_packet_derive(input: DeriveInput) -> TokenStream {
    let packet_id = util::parse_packet_id(input.attrs);
    let struct_ident = input.ident;

    let mut parts = Vec::new();

    if let Some(id) = packet_id {
        parts.push(quote! { buf.put_u8(#id); });
    }

    if let Data::Struct(_struct) = input.data {
        if let Fields::Named(named_fields) = _struct.fields {
            for field in named_fields.named.into_iter() {
                let ident = field.ident.expect("ident").clone();
                let ty_name = util::parse_type(&field.ty).expect("Type name");
                let fn_name = name_for_ty(&ty_name.to_string());
                let part = if let Some(func) = fn_name {
                    quote! {
                        buf.#func(self.#ident);
                    }
                }else {
                    quote! {
                        ::nt_packet::ClientMessage::encode(&self.#ident, buf);
                    }
                };
                parts.push(part);
            }
        }
    }

    quote! {
        impl ::nt_packet::ClientMessage for #struct_ident {
            fn encode(&self, buf: &mut ::bytes::BytesMut) {
                use ::bytes::BufMut;
                #(#parts)*
            }
        }
    }
}


fn name_for_ty(ty_name: &str) -> Option<Ident> {
    match ty_name {
        "u8" => Some(Ident::new("put_u8", Span::call_site())),
        "i8" => Some(Ident::new("put_i8", Span::call_site())),
        "u16" => Some(Ident::new("put_u16_be", Span::call_site())),
        "i16" => Some(Ident::new("put_i16_be", Span::call_site())),
        "u32" => Some(Ident::new("put_u32_be", Span::call_site())),
        "i32" => Some(Ident::new("put_i32_be", Span::call_site())),
        "u64" => Some(Ident::new("put_u64_be", Span::call_site())),
        "i64" => Some(Ident::new("put_i64_be", Span::call_site())),
        _ => None
    }
}
