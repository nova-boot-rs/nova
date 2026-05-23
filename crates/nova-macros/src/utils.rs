use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

pub fn rest_controller(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        #input

        impl #name {
            pub fn nova_metadata() -> &'static str {
                "Registered as Nova Controller"
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn request_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        impl ::nova_core::NovaRequestModel for #name {}
    };

    TokenStream::from(expanded)
}

pub fn response_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        impl ::nova_core::NovaResponseModel for #name {}
    };

    TokenStream::from(expanded)
}
