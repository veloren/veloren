extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct};

#[proc_macro_attribute]
pub fn global_state(_args: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as ItemStruct);
    let name = &parsed.ident;
    let out: proc_macro2::TokenStream = quote! {
        #parsed
        type PLUGIN_STATE_TYPE = #name;

        static mut PLUGIN_STATE: Option<PLUGIN_STATE_TYPE> = None;

        static PLUGIN_STATE_GUARD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    };
    out.into()
}

#[proc_macro_attribute]
pub fn event_handler(_args: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(item as ItemFn);
    let fn_body = parsed.block; // function body
    let sig = parsed.sig; // function signature
    let fn_name = sig.ident; // function name/identifier
    let fn_args = sig.inputs; // comma separated args
    let fn_return = sig.output; // comma separated args

    let out: proc_macro2::TokenStream = if fn_args.len() == 1 {
        quote! {
            #[no_mangle]
            pub fn #fn_name(intern__ptr: i64, intern__len: i64) -> i64 {
                let input = ::veloren_plugin_rt::read_input(intern__ptr as _,intern__len as _).unwrap();
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
        }
    } else {
        quote! {
            #[no_mangle]
            pub fn #fn_name(intern__ptr: i64, intern__len: i64) -> i64 {
                let input = ::veloren_plugin_rt::read_input(intern__ptr as _,intern__len as _).unwrap();
                #[inline]
                fn inner(#fn_args) #fn_return {
                    #fn_body
                }
                // Artificially force the event handler to be type-correct
                fn force_event<E: ::veloren_plugin_rt::api::Event>(event: E, inner: fn(E, &mut PLUGIN_STATE_TYPE) -> E::Response) -> E::Response {
                    //let mut plugin_state = PLUGIN_STATE.lock().unwrap();

                        assert_eq!(PLUGIN_STATE_GUARD.swap(true, std::sync::atomic::Ordering::Acquire), false);
                        unsafe {
                            if PLUGIN_STATE.is_none() {
                                PLUGIN_STATE = Some(PLUGIN_STATE_TYPE::default());
                            }
                        }
                        let out = inner(event, unsafe {PLUGIN_STATE.as_mut().unwrap()});
                        PLUGIN_STATE_GUARD.store(false, std::sync::atomic::Ordering::Release);
                        out

                }
                ::veloren_plugin_rt::write_output(&force_event(input, inner))
            }
        }
    };
    out.into()
}
