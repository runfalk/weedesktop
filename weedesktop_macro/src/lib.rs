extern crate proc_macro;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;
use std::ffi::CString;
use syn::{Expr, Ident, ItemFn, LitInt, LitStr, LitByteStr, ItemStatic};
use syn::export::{Span, ToTokens};
use syn::parse::{Parse, ParseStream, Result, Error};

mod ffi;

#[derive(Debug)]
enum PluginInfo {
    Name(String),
    Description(String),
    Author(String),
    Version(String),
    License(String),
    Priority(i32),
}

fn parse_expr<T: Parse>(expr: Expr) -> Result<T> {
    Ok(syn::parse::<T>(expr.into_token_stream().into())?)
}

impl Parse for PluginInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        use PluginInfo::*;

        let static_item = input.parse::<ItemStatic>()?;
        // TODO: ensure pub
        if !input.is_empty() {
            return Err(input.error("Unexpected tokens"))
        }

        if let Some(mutability) = static_item.mutability {
            return Err(Error::new(mutability.span, "Plugin info must not be mutable"));
        }

        // Ensure we have a valid ident
        match static_item.ident.to_string().as_str() {
            "NAME" => {
                Ok(Name(parse_expr::<LitStr>(*static_item.expr)?.value()))
            },
            "DESCRIPTION" => {
                Ok(Description(parse_expr::<LitStr>(*static_item.expr)?.value()))
            },
            "AUTHOR" => {
                Ok(Author(parse_expr::<LitStr>(*static_item.expr)?.value()))
            },
            "LICENSE" => {
                Ok(License(parse_expr::<LitStr>(*static_item.expr)?.value()))
            },
            "VERSION" => {
                Ok(Version(parse_expr::<LitStr>(*static_item.expr)?.value()))
            },
            "PRIORITY" => {
                Ok(Priority(parse_expr::<LitInt>(*static_item.expr)?.value() as i32))
            },
            _ => return Err(Error::new(static_item.ident.span(), "Unexpected name")),
        }
    }
}

fn export_string(name: &str, value: &str) -> TokenStream {
    let ident = Ident::new(name, Span::call_site());
    let bytes = CString::new(value).unwrap().into_bytes_with_nul();
    let bytes_len = bytes.len();
    let lit_byte_str = LitByteStr::new(&bytes, Span::call_site());

    TokenStream::from(quote! {
        #[no_mangle]
        pub static #ident: [u8; #bytes_len] = *#lit_byte_str;
    })
}

#[proc_macro_attribute]
pub fn plugin_info(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let plugin_info = parse_macro_input!(input as PluginInfo);

    let expanded = match plugin_info {
        PluginInfo::Name(name) => export_string("weechat_plugin_name", &name),
        PluginInfo::Description(desc) => export_string("weechat_plugin_description", &desc),
        PluginInfo::Author(author) => export_string("weechat_plugin_author", &author),
        PluginInfo::Version(version) => export_string("weechat_plugin_version", &version),
        PluginInfo::License(license) => export_string("weechat_plugin_license", &license),
        PluginInfo::Priority(priority) => TokenStream::from(quote! {
            #[no_mangle]
            pub static weechat_plugin_priority: i32 = #priority;
        }),
    };
    expanded
}

#[proc_macro_attribute]
pub fn plugin_init(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let init_fn = parse_macro_input!(input as ItemFn);
    let api_version = LitByteStr::new(ffi::WEECHAT_PLUGIN_API_VERSION, Span::call_site());
    let api_version_len = ffi::WEECHAT_PLUGIN_API_VERSION.len();
    let init_fn_name = init_fn.ident.clone();

    TokenStream::from(quote! {
        #[no_mangle]
        pub static weechat_plugin_api_version: [u8; #api_version_len] = *#api_version;

        #init_fn

        #[no_mangle]
        pub extern "C" fn weechat_plugin_init(ptr: *mut ffi::t_weechat_plugin, _argc: i32, _argv: *const *const u8) -> i32 {
            match #init_fn_name(&weechat::Plugin::new(ptr)) {
                Ok(()) => ffi::WEECHAT_RC_OK,
                Err(()) => ffi::WEECHAT_RC_ERROR,
            }
        }
    })
}

#[proc_macro_attribute]
pub fn plugin_end(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let end_fn = parse_macro_input!(input as ItemFn);
    let end_fn_name = end_fn.ident.clone();

    TokenStream::from(quote! {
        #end_fn

        #[no_mangle]
        pub extern "C" fn weechat_plugin_end(ptr: *mut ffi::t_weechat_plugin) -> i32 {
            match #end_fn_name(&weechat::Plugin::new(ptr)) {
                Ok(()) => ffi::WEECHAT_RC_OK,
                Err(()) => ffi::WEECHAT_RC_ERROR,
            }
        }
    })
}
