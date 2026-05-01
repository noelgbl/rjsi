//! `JsClass` / `#[js_methods]` — only reference [`rjsi_core`] (no engine
//! crates).

mod attrs;

use attrs::{JsMethodsAttrs, parse_js_class_on_struct, strip_js_attrs_from_impl};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
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

    quote! {
        unsafe impl #core::NativeClass for #ident {
            const NAME: &'static str = #name_lit;

            fn descriptor<R: #core::Runtime>() -> &'static #core::ClassDescriptor<R> {
                #core::class_descriptor::<R, Self>(Self::__rjsi_build_class_descriptor::<R>)
            }
        }
    }
}

pub fn expand_js_methods(attr: TokenStream2, mut input: ItemImpl) -> TokenStream2 {
    let core = core_path();
    let _opts: JsMethodsAttrs = match syn::parse2(attr) {
        Ok(o) => o,
        Err(e) => return e.to_compile_error(),
    };

    input = strip_js_attrs_from_impl(input);

    let self_ty = &input.self_ty;
    let ty_stub = type_ident_stub(self_ty);
    let fin = format_ident!("__rjsi_native_drop_{}", ty_stub, span = Span::call_site());

    let build_fn: syn::ImplItemFn = syn::parse_quote! {
        #[doc(hidden)]
        fn __rjsi_build_class_descriptor<R: #core::Runtime>() -> #core::ClassDescriptor<R> {
            unsafe fn #fin(p: *mut ::std::ffi::c_void) {
                drop(::std::boxed::Box::from_raw(p.cast::<#self_ty>()));
            }

            #core::ClassDescriptor {
                name: <Self as #core::NativeClass>::NAME,
                constructor: None,
                methods: &[],
                statics: &[],
                accessors: &[],
                symbols: &[],
                finalizer: #fin,
            }
        }
    };

    input.items.push(syn::ImplItem::Fn(build_fn));

    quote! { #input }
}

fn type_ident_stub(ty: &syn::Type) -> String {
    let raw = ty.to_token_stream().to_string();
    let mut s = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else {
            s.push('_');
        }
    }
    if s.is_empty() {
        s.push('T');
    }
    s.truncate(64);
    s
}
