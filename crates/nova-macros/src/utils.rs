//! Utility helpers and derive implementations used by the public macros.
//!
//! These helpers implement small code-generation rules such as deriving the
//! `NovaRequest`/`NovaResponse` marker traits and adding controller
//! metadata helpers.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

/// Implements a small metadata helper on controller structs. The macro
/// preserves the original struct and adds a `nova_metadata()` method used
/// by examples and tests.
pub fn rest_controller(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        #input

        impl #name {
            /// Returns a short marker string describing the controller.
            pub fn nova_metadata() -> &'static str {
                "Registered as Nova Controller"
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive implementation that marks the struct as a request model by
/// implementing the `NovaRequestModel` marker trait.
pub fn request_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        impl ::nova_core::NovaRequestModel for #name {}
    };

    TokenStream::from(expanded)
}

/// Derive implementation that marks the struct as a response model by
/// implementing the `NovaResponseModel` marker trait.
pub fn response_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        impl ::nova_core::NovaResponseModel for #name {}
    };

    TokenStream::from(expanded)
}
