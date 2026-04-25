use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Meta, parse::Parse, punctuated::Punctuated};

#[derive(Default)]
struct JsExportOptions {
    clone: bool,
}

impl Parse for JsExportOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut options = Self::default();
        let metas = Punctuated::<Meta, syn::Token![,]>::parse_terminated(input)?;
        for meta in metas {
            match meta {
                Meta::Path(path) if path.is_ident("clone") => {
                    options.clone = true;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "unsupported #[js_export] option; expected `clone`",
                    ));
                }
            }
        }
        Ok(options)
    }
}

/// Main implementation of the object macro
pub fn class_instance_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let type_name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let data = &input.data;
    let options = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("js_export"))
        .map(|attr| attr.parse_args::<JsExportOptions>())
        .transpose()?
        .unwrap_or_default();

    // Filter out object attributes
    let filtered_attrs: Vec<_> = input
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("js_export"))
        .collect();

    // Rebuild type definition with filtered attributes
    let type_def = match data {
        syn::Data::Struct(s) => {
            let fields = &s.fields;
            quote! {
                #(#filtered_attrs)*
                #[derive(Clone)]
                #vis struct #type_name #generics #fields
            }
        }
        _ => return Err(syn::Error::new_spanned(input, "Only structs are supported")),
    };

    let clone_from_js_impl = if options.clone {
        quote! {
            impl rjsi::FromJSValue<rjsi::JSEngineValue> for #type_name {
                fn from_js_value(ctx: &rjsi::JSContext, value: rjsi::JSValue) -> rjsi::JSResult<Self> {
                    let obj = rjsi::JSObject::from_js_value(ctx, value)?;
                    let instance = obj.borrow::<Self>()?;
                    // Some JS-exposed structs implement an inherent `clone()` method (e.g. `Response.prototype.clone()`).
                    // Method-call syntax would prefer the inherent method over `Clone::clone`, which would break
                    // internal "this" passing by returning a fresh value instead of a plain Rust clone.
                    Ok(<Self as ::core::clone::Clone>::clone(&*instance))
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #type_def

        impl rjsi::IntoJSValue<rjsi::JSEngineValue> for #type_name {
            fn into_js_value(self, context: &rjsi::JSContext) -> rjsi::JSValue {
                rjsi::Class::lookup::<Self>(context)
                    .map(|class| class.instance(self).into_js_value())
                    .unwrap_or_else(|_| context.throw_error("Failed to make Class Instance"))
            }
        }

        #clone_from_js_impl

        impl rjsi::function::JSParameterType for #type_name {}
    };

    Ok(expanded)
}
