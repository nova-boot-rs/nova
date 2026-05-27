//! Procedural macros used by Nova for defining REST controllers and
//! request/response models. These helpers provide attribute macros like
//! `get`, `post`, `rest_controller` and derive macros `NovaRequest` and
//! `NovaResponse` used throughout the example and plugin crates.

use proc_macro::TokenStream;

mod routing;
mod utils;

#[allow(dead_code)]
mod validation;

/// Marks a struct as a Nova controller. The macro currently adds a
/// `nova_metadata()` helper; route handlers are attached via `get/post/...`
/// attributes on impl functions.
#[proc_macro_attribute]
pub fn rest_controller(_args: TokenStream, input: TokenStream) -> TokenStream {
    utils::rest_controller(input)
}

/// Attach a GET route to a free function. Usage: `#[get("/path")]`.
#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::get(args, input)
}

/// Attach a POST route to a free function. Usage: `#[post("/path")]`.
#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::post(args, input)
}

/// Attach a PUT route to a free function. Usage: `#[put("/path")]`.
#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::put(args, input)
}

/// Attach a DELETE route to a free function. Usage: `#[delete("/path")]`.
#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::delete(args, input)
}

/// Attach a PATCH route to a free function. Usage: `#[patch("/path")]`.
#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::patch(args, input)
}

/// Derive macro to mark a type as a Nova request model.
#[proc_macro_derive(NovaRequest)]
pub fn request_model(input: TokenStream) -> TokenStream {
    utils::request_model(input)
}

/// Derive macro to mark a type as a Nova response model.
#[proc_macro_derive(NovaResponse)]
pub fn response_model(input: TokenStream) -> TokenStream {
    utils::response_model(input)
}
