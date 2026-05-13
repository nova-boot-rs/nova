use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemStruct, parse_macro_input};

fn route_macro(method: &str, path: String, handler: syn::ItemFn) -> TokenStream {
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
    route_macro(
        "GET",
        parse_macro_input!(args as syn::LitStr).value(),
        parse_macro_input!(input as syn::ItemFn),
    )
}

// post macro: #[post("/path")]
#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "POST",
        parse_macro_input!(args as syn::LitStr).value(),
        parse_macro_input!(input as syn::ItemFn),
    )
}

// put macro: #[put("/path")]
#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "PUT",
        parse_macro_input!(args as syn::LitStr).value(),
        parse_macro_input!(input as syn::ItemFn),
    )
}

// delete macro: #[delete("/path")]
#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "DELETE",
        parse_macro_input!(args as syn::LitStr).value(),
        parse_macro_input!(input as syn::ItemFn),
    )
}

// patch macro: #[patch("/path")]
#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    route_macro(
        "PATCH",
        parse_macro_input!(args as syn::LitStr).value(),
        parse_macro_input!(input as syn::ItemFn),
    )
}

#[proc_macro_derive(NovaRequest)]
pub fn request_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        impl ::nova_core::NovaRequestModel for #name {}
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(NovaResponse)]
pub fn response_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;

    let expanded = quote! {
        impl ::nova_core::NovaResponseModel for #name {}
    };

    TokenStream::from(expanded)
}
