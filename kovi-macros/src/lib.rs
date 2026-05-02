use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;

    let expanded = quote! {
        #input

        #[doc(hidden)]
        pub fn __kovi_get_plugin_info() -> (&'static str, &'static str) {
            let name = env!("CARGO_PKG_NAME");
            let version = env!("CARGO_PKG_VERSION");
            (name, version)
        }

        #[doc(hidden)]
        pub fn __kovi_run_async_plugin() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        {
            Box::pin(async {
                #fn_name().await;
            })
        }

        #[doc(hidden)]
        pub fn __kovi_build_plugin() -> kovi::plugin::Plugin {
            let (name, version) = crate::__kovi_get_plugin_info();

            kovi::plugin::Plugin::new(
                name,
                version,
                std::sync::Arc::new(crate::__kovi_run_async_plugin),
            )
        }

        #[doc(hidden)]
        pub fn build_plugin() -> kovi::plugin::Plugin {
            let (name, version) = crate::__kovi_get_plugin_info();

            kovi::plugin::Plugin::new(
                name,
                version,
                std::sync::Arc::new(crate::__kovi_run_async_plugin),
            )
        }
    };

    TokenStream::from(expanded)
}
