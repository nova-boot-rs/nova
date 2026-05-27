//! Route attribute macros used to register functions as HTTP handlers.
//!
//! Each attribute (e.g., `#[get("/path")]`) expands the function and
//! submits a `NovaRoute` entry into the `inventory` collector so the
//! framework can register the handler at startup.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

/// Internal helper that builds the inventory registration for a route.
fn route_macro(method: &str, path: String, handler: ItemFn) -> TokenStream {
    let fn_name = &handler.sig.ident;
    let route_expr = match method {
        "GET" => quote!(::nova_boot::axum::routing::get(#fn_name)),
        "POST" => quote!(::nova_boot::axum::routing::post(#fn_name)),
        "PUT" => quote!(::nova_boot::axum::routing::put(#fn_name)),
        "DELETE" => quote!(::nova_boot::axum::routing::delete(#fn_name)),
        "PATCH" => quote!(::nova_boot::axum::routing::patch(#fn_name)),
        _ => quote!(::nova_boot::axum::routing::get(#fn_name)),
    };

    let expanded = quote! {
        #handler

        ::nova_boot::inventory::submit! {
            ::nova_boot::NovaRoute {
                path: #path,
                method: #method,
                handler: || #route_expr,
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for `GET` handlers.
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "GET",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}

/// Attribute macro for `POST` handlers.
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "POST",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}

/// Attribute macro for `PUT` handlers.
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "PUT",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}

/// Attribute macro for `DELETE` handlers.
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "DELETE",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}

/// Attribute macro for `PATCH` handlers.
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "PATCH",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}
