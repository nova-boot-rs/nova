use proc_macro::TokenStream;

mod routing;
mod utils;

#[allow(dead_code)]
mod validation;

#[proc_macro_attribute]
pub fn rest_controller(_args: TokenStream, input: TokenStream) -> TokenStream {
    utils::rest_controller(input)
}

#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::get(args, input)
}

#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::post(args, input)
}

#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::put(args, input)
}

#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::delete(args, input)
}

#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    routing::patch(args, input)
}

#[proc_macro_derive(NovaRequest)]
pub fn request_model(input: TokenStream) -> TokenStream {
    utils::request_model(input)
}

#[proc_macro_derive(NovaResponse)]
pub fn response_model(input: TokenStream) -> TokenStream {
    utils::response_model(input)
}
