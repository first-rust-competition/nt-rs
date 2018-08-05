extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro2;

mod client;
mod server;
mod util;

use proc_macro::TokenStream;
use proc_macro2::Span;

#[proc_macro_derive(ClientMessage, attributes(packet_id))]
pub fn nt_client_packet_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    let gen = client::gen_client_packet_derive(ast);

    gen.into()
}

#[proc_macro_derive(ServerMessage, attributes(packet_id))]
pub fn nt_server_packet_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input) .unwrap();

    let gen = server::gen_server_packet_derive(ast);

    gen.into()
}
