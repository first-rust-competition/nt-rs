use super::util;
use proc_macro2::{TokenStream, Span};
use syn::{DeriveInput, DataStruct, Data, Fields, FieldsNamed, Ident};


pub fn gen_server_packet_derive(input: DeriveInput) -> TokenStream {
    let struct_ident = input.ident;

    let mut parts = Vec::new();
    let mut idents = Vec::new();

    if let Data::Struct(_struct) = input.data {
        if let Fields::Named(named_fields) = _struct.fields {
            for field in named_fields.named.into_iter() {
                let ident = field.ident.unwrap().clone();
                let ty_name = util::parse_type(&field.ty).unwrap();
                let fn_name = name_for_ty(&ty_name.to_string());
                let part = if let Some(func) = fn_name {
                    quote! {
                        let #ident = buf.#func();
                    }
                } else {
                    quote! {
                        let #ident = <#ty_name as ::nt_packet::ServerMessage>::decode(buf).unwrap();
                    }
                };
                parts.push(part);
                idents.push(quote!(#ident));
            }
        }
    }

    quote! {
        impl ::nt_packet::ServerMessage for #struct_ident {
            fn decode(buf: &mut ::bytes::Buf) -> Option<Self> {
                //TODO
                #(#parts)*

                Some(#struct_ident {
                    #(#idents,)*
                })
            }
        }
    }
}

pub fn name_for_ty(ty_name: &str) -> Option<Ident> {
    match ty_name {
        "u8" => Some(Ident::new("get_u8", Span::call_site())),
        "i8" => Some(Ident::new("get_i8", Span::call_site())),
        "u16" => Some(Ident::new("get_u16_be", Span::call_site())),
        "i16" => Some(Ident::new("get_i16_be", Span::call_site())),
        "u32" => Some(Ident::new("get_u32_be", Span::call_site())),
        "i32" => Some(Ident::new("get_i32_be", Span::call_site())),
        "u64" => Some(Ident::new("get_u64_be", Span::call_site())),
        "i64" => Some(Ident::new("get_i64_be", Span::call_site())),
        _ => None
    }
}