use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Index, ItemImpl, parse_macro_input};

mod class;

pub(crate) fn core_path() -> TokenStream2 {
    match crate_name("rjsi-core") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::rjsi_core),
    }
}

#[proc_macro_derive(JsClass, attributes(js_class))]
pub fn derive_js_class(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    class::expand_js_class(input).into()
}

#[proc_macro_attribute]
pub fn js_methods(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_ts = proc_macro2::TokenStream::from(attr);
    let impl_block = parse_macro_input!(item as ItemImpl);
    class::expand_js_methods(attr_ts, impl_block).into()
}

#[proc_macro_derive(IntoJs)]
pub fn derive_into_js(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_into_js(&input).into()
}

#[proc_macro_derive(FromJs)]
pub fn derive_from_js(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_from_js(&input).into()
}

/// `proc_macro_crate` returns `Itself` for all targets in the `rjsi` package;
/// expanding to `crate` breaks examples, which are separate crates. Emit
/// `::rjsi` for that case: the `rjsi` crate re-exports `Runtime`, `HostError`,
/// and the ser traits, and the path still resolves from the `rjsi` library
/// itself.
fn runtime_path() -> TokenStream2 {
    match crate_name("rjsi") {
        Ok(FoundCrate::Itself) => quote!(::rjsi),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Err(_) => match crate_name("rjsi-core") {
            Ok(FoundCrate::Itself) => quote!(crate),
            Ok(FoundCrate::Name(name)) => {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#ident)
            }
            Err(_) => quote!(::rjsi),
        },
    }
}

fn expand_into_js(input: &DeriveInput) -> TokenStream2 {
    let path = runtime_path();
    let ident = &input.ident;
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let setters = fields.named.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap();
                    let ty = &field.ty;
                    let key = name.to_string();
                    quote! {
                        let value = <#ty as #path::IntoJs<'s, R>>::into_js(self.#name, scope)?;
                        #path::ValueLike::set(&object, scope, #key, value);
                    }
                });
                quote! {
                    impl<'s, R> #path::IntoJs<'s, R> for #ident
                    where
                        R: #path::Runtime,
                    {
                        fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
                            let object = #path::ScopeLike::object(scope);
                            #( #setters )*
                            Ok(object)
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let len = fields.unnamed.len() as u32;
                let setters = fields.unnamed.iter().enumerate().map(|(index, field)| {
                    let idx = Index::from(index);
                    let ty = &field.ty;
                    let index_u32 = index as u32;
                    quote! {
                        let value = <#ty as #path::IntoJs<'s, R>>::into_js(self.#idx, scope)?;
                        #path::ValueLike::set_index(&array, scope, #index_u32, value);
                    }
                });
                quote! {
                    impl<'s, R> #path::IntoJs<'s, R> for #ident
                    where
                        R: #path::Runtime,
                    {
                        fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
                            let array = #path::ScopeLike::array(scope, #len);
                            #( #setters )*
                            Ok(array)
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<'s, R> #path::IntoJs<'s, R> for #ident
                where
                    R: #path::Runtime,
                {
                    fn into_js(self, scope: &mut R::Scope<'s, '_>) -> Result<R::Value<'s>, R::Error> {
                        Ok(#path::ScopeLike::object(scope))
                    }
                }
            },
        },
        _ => quote!(compile_error!("IntoJs can only be derived for structs")),
    }
}

fn expand_from_js(input: &DeriveInput) -> TokenStream2 {
    let path = runtime_path();
    let ident = &input.ident;
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let bindings = fields.named.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap();
                    let key = name.to_string();
                    quote! {
                        let #name = #path::ValueLike::get(&value, scope, #key);
                    }
                });
                let readers = fields.named.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap();
                    let ty = &field.ty;
                    quote! {
                        #name: <#ty as #path::FromJs<'s, R>>::from_js(scope, #name)?
                    }
                });
                quote! {
                    impl<'s, R> #path::FromJs<'s, R> for #ident
                    where
                        R: #path::Runtime,
                    {
                        fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error> {
                            if !#path::ValueLike::is_object(&value) {
                                return Err(#path::HostError::type_error(#path::E_TYPE, "expected object").into());
                            }
                            #( #bindings )*
                            Ok(Self { #( #readers, )* })
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let bindings = fields.unnamed.iter().enumerate().map(|(index, _)| {
                    let binding =
                        syn::Ident::new(&format!("field_{index}"), proc_macro2::Span::call_site());
                    let index_u32 = index as u32;
                    quote! {
                        let #binding = #path::ValueLike::get_index(&value, scope, #index_u32);
                    }
                });
                let readers = fields.unnamed.iter().enumerate().map(|(index, field)| {
                    let binding =
                        syn::Ident::new(&format!("field_{index}"), proc_macro2::Span::call_site());
                    let ty = &field.ty;
                    quote! {
                        <#ty as #path::FromJs<'s, R>>::from_js(scope, #binding)?
                    }
                });
                quote! {
                    impl<'s, R> #path::FromJs<'s, R> for #ident
                    where
                        R: #path::Runtime,
                    {
                        fn from_js(scope: &mut R::Scope<'s, '_>, value: R::Value<'s>) -> Result<Self, R::Error> {
                            if !#path::ValueLike::is_array(&value) {
                                return Err(#path::HostError::type_error(#path::E_TYPE, "expected array").into());
                            }
                            #( #bindings )*
                            Ok(Self( #( #readers, )* ))
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<'s, R> #path::FromJs<'s, R> for #ident
                where
                    R: #path::Runtime,
                {
                    fn from_js(_scope: &mut R::Scope<'s, '_>, _value: R::Value<'s>) -> Result<Self, R::Error> {
                        Ok(Self)
                    }
                }
            },
        },
        _ => quote!(compile_error!("FromJs can only be derived for structs")),
    }
}
