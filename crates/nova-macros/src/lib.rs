use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

// rest_controller macro: #[rest_controller]
#[proc_macro_attribute]
pub fn rest_controller(_args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the struct the user put the attribute on
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        #input

        impl #name {
            // This is where we could add common methods or traits for all controllers in the future
            // For example, a default constructor or shared functionality
            pub fn nova_metadata() -> &'static str {
                "Registered as Nova Controller"
            }
        }
    };

    TokenStream::from(expanded)
}

// get macro: #[get("/path")]
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

// post macro: #[post("/path")]
#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(args as syn::LitStr).value();
    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;

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

// put macro: #[put("/path")]
#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(args as syn::LitStr).value();
    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;

    let expanded = quote! {
        #input_fn

        // Submit this route to the Nova inventory at compile time
        ::nova_core::inventory::submit! {
            ::nova_core::NovaRoute {
                path: #path,
                method: "PUT",
                handler: || ::nova_core::axum::routing::put(#fn_name),
            }
        }
    };
    TokenStream::from(expanded)
}

// delete macro: #[delete("/path")]
#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(args as syn::LitStr).value();
    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;

    let expanded = quote! {
        #input_fn

        // Submit this route to the Nova inventory at compile time
        ::nova_core::inventory::submit! {
            ::nova_core::NovaRoute {
                path: #path,
                method: "DELETE",
                handler: || ::nova_core::axum::routing::delete(#fn_name),
            }
        }
    };
    TokenStream::from(expanded)
}

// patch macro: #[patch("/path")]
#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(args as syn::LitStr).value();
    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;

    let expanded = quote! {
        #input_fn

        // Submit this route to the Nova inventory at compile time
        ::nova_core::inventory::submit! {
            ::nova_core::NovaRoute {
                path: #path,
                method: "PATCH",
                handler: || ::nova_core::axum::routing::patch(#fn_name),
            }
        }
    };
    TokenStream::from(expanded)
}

