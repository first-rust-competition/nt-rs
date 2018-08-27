use super::util;
use proc_macro2::{TokenStream, Span};
use syn::{DeriveInput, Data, Fields, Ident};


pub fn gen_server_packet_derive(input: DeriveInput) -> TokenStream {
    let struct_ident = input.ident;

    let mut parts = Vec::new();
    let mut idents = Vec::new();

    if let Data::Struct(_struct) = input.data {
        if let Fields::Named(named_fields) = _struct.fields {
            for field in named_fields.named.into_iter() {
                let ident = field.ident.unwrap().clone();
                let ty_name = util::parse_type(&field.ty).unwrap();
                let (fn_name, bytes) = name_for_ty(&ty_name.to_string());
                let part = if let Some(func) = fn_name {
                    quote! {
                        let #ident = buf.#func()?;
                        bytes_read += #bytes;
                    }
                } else {
                    quote! {
                        let #ident = {
                            let (a, b) = <#ty_name as ::nt_packet::ServerMessage>::decode(buf)?;
                            bytes_read += b;
                            a
                        };
                    }
                };
                parts.push(part);
                idents.push(quote!(#ident));
            }
        }
    }

    quote! {
        impl ::nt_packet::ServerMessage for #struct_ident {
            fn decode(mut buf: &mut ::bytes::Buf) -> ::std::result::Result<(Self, usize), ::failure::Error> {
                let mut bytes_read = 0usize;
                #(#parts)*

                Ok((#struct_ident {
                    #(#idents,)*
                }, bytes_read))
            }
        }
    }
}

pub fn name_for_ty(ty_name: &str) -> (Option<Ident>, usize) {
    match ty_name {
        "u8" => (Some(Ident::new("read_u8", Span::call_site())), 1),
        "i8" => (Some(Ident::new("read_i8", Span::call_site())), 1),
        "u16" => (Some(Ident::new("read_u16_be", Span::call_site())), 2),
        "i16" => (Some(Ident::new("read_i16_be", Span::call_site())), 2),
        "u32" => (Some(Ident::new("read_u32_be", Span::call_site())), 4),
        "i32" => (Some(Ident::new("read_i32_be", Span::call_site())), 4),
        "u64" => (Some(Ident::new("read_u64_be", Span::call_site())), 8),
        "i64" => (Some(Ident::new("read_i64_be", Span::call_site())), 8),
        _ => (None, 0)
    }
}