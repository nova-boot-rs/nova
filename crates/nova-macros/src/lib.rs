use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

#[proc_macro_attribute]
pub fn rest_controller(_args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the struct the user put the attribute on
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    // For now, we simply pass the struct back through.
    // Later, this macro will generate a "register" function for this struct.
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

#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(args as syn::LitStr).value(); 
    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;

    let expanded = quote! {
        #input_fn

        // Submit this route to the Nova inventory at compile time
        ::nova_core::inventory::submit! {
            ::nova_core::NovaRoute {
                path: #path,
                method: "GET",
                handler: || ::nova_core::axum::routing::get(#fn_name),
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(args as syn::LitStr).value();
    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;

    // This is a simplified version. In a real framework,
    // you would store this path metadata to be used by NovaApp.
    let expanded = quote! {
        #input_fn

        // Submit this route to the Nova inventory at compile time
        ::nova_core::inventory::submit! {
            ::nova_core::NovaRoute {
                path: #path,
                method: "POST",
                handler: || ::nova_core::axum::routing::post(#fn_name),
            }
        }
    };
    TokenStream::from(expanded)
}
