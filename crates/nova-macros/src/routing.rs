use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

fn route_macro(method: &str, path: String, handler: ItemFn) -> TokenStream {
    let fn_name = &handler.sig.ident;
    let route_expr = match method {
        "GET" => quote!(::nova_core::axum::routing::get(#fn_name)),
        "POST" => quote!(::nova_core::axum::routing::post(#fn_name)),
        "PUT" => quote!(::nova_core::axum::routing::put(#fn_name)),
        "DELETE" => quote!(::nova_core::axum::routing::delete(#fn_name)),
        "PATCH" => quote!(::nova_core::axum::routing::patch(#fn_name)),
        _ => quote!(::nova_core::axum::routing::get(#fn_name)),
    };

    let expanded = quote! {
        #handler

        ::nova_core::inventory::submit! {
            ::nova_core::NovaRoute {
                path: #path,
                method: #method,
                handler: || #route_expr,
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "GET",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}

pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "POST",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}

pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "PUT",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}

pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "DELETE",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}

pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "PATCH",
        parse_macro_input!(args as LitStr).value(),
        parse_macro_input!(input as ItemFn),
    )
}
