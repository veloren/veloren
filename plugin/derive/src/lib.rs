extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn event_handler(_args: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as ItemFn);
    let fn_body = parsed.block; // function body
    let sig = parsed.sig; // function signature
    let fn_name = sig.ident; // function name/identifier
    let fn_args = sig.inputs; // comma separated args
    let fn_return = sig.output; // comma separated args

    let out: proc_macro2::TokenStream = quote! {
        #[allow(clippy::unnecessary_wraps)]
        #[no_mangle]
        pub fn #fn_name(intern__ptr: i32, intern__len: u32) -> i32 {
            let input = ::veloren_plugin_rt::read_input(intern__ptr,intern__len).unwrap();
            #[inline]
            fn inner(#fn_args) #fn_return {
                #fn_body
            }
            // Artificially force the event handler to be type-correct
            fn force_event<E: ::veloren_plugin_rt::api::Event>(event: E, inner: fn(E) -> E::Response) -> E::Response {
                inner(event)
            }
            ::veloren_plugin_rt::write_output(&force_event(input, inner))
        }
    };
    out.into()
}
