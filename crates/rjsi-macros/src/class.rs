mod attrs;

use attrs::{JsMethodsAttrs, parse_js_class_on_struct, strip_js_attrs_from_impl};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{DeriveInput, ItemImpl};

use crate::core_path;

pub fn expand_js_class(input: DeriveInput) -> TokenStream2 {
    let core = core_path();
    let ident = &input.ident;
    let attrs = match parse_js_class_on_struct(&input.attrs) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };
    let name_lit = attrs
        .name
        .as_ref()
        .map(|s| syn::LitStr::new(s, Span::call_site()))
        .unwrap_or_else(|| syn::LitStr::new(&ident.to_string(), Span::call_site()));
    let constructor_body = if attrs.no_constructor {
        quote! {
            Err(#core::JsError::type_err("constructor disabled"))
        }
    } else {
        quote! {
            Err(#core::JsError::type_err("constructor not implemented"))
        }
    };

    quote! {
        impl<E: #core::Engine> #core::JsClass<E> for #ident {
            const NAME: &'static str = #name_lit;

            fn prototype<'cx>(
                _cx: &mut #core::Context<'cx, E>,
                _proto: E::Object<'cx>,
            ) -> #core::JsResult<'cx, E, ()> {
                Ok(())
            }

            fn constructor<'cx, 'rt>(
                _cx: &mut #core::CallbackCx<'cx, 'rt, E>,
                _args: #core::Args<'rt, E>,
            ) -> #core::JsResult<'rt, E, Self> {
                #constructor_body
            }
        }
    }
}

pub fn expand_js_methods(attr: TokenStream2, mut input: ItemImpl) -> TokenStream2 {
    let _opts: JsMethodsAttrs = match syn::parse2(attr) {
        Ok(o) => o,
        Err(e) => return e.to_compile_error(),
    };

    input = strip_js_attrs_from_impl(input);
    quote! { #input }
}
