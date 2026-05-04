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
        Err(_) => match crate_name("rjsi") {
            Ok(FoundCrate::Itself) => quote!(crate),
            Ok(FoundCrate::Name(name)) => {
                let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
                quote!(::#ident)
            }
            Err(_) => quote!(::rjsi_core),
        },
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

#[proc_macro_attribute]
pub fn js_constructor(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn js_static(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn js_get(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn js_set(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn js_skip(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
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
                    let key = name.to_string();
                    quote! {
                        object.set_typed(cx, #key, self.#name)?;
                    }
                });
                quote! {
                    impl<'cx, E> #path::ToJs<'cx, E> for #ident
                    where
                        E: #path::Engine,
                    {
                        fn to_js(self, cx: &mut #path::Context<'cx, E>) -> #path::Result<E::Value<'cx>> {
                            let object = cx.new_object()?;
                            #( #setters )*
                            Ok(object.into_value().into_raw())
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let setters = fields.unnamed.iter().enumerate().map(|(index, _field)| {
                    let idx = Index::from(index);
                    let index_u32 = index as u32;
                    quote! {
                        array.set_typed(cx, #index_u32, self.#idx)?;
                    }
                });
                quote! {
                    impl<'cx, E> #path::ToJs<'cx, E> for #ident
                    where
                        E: #path::Engine,
                    {
                        fn to_js(self, cx: &mut #path::Context<'cx, E>) -> #path::Result<E::Value<'cx>> {
                            let array_value = cx.eval("[]")?;
                            let array = array_value.try_as_object()?;
                            #( #setters )*
                            Ok(array.into_value().into_raw())
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<'cx, E> #path::ToJs<'cx, E> for #ident
                where
                    E: #path::Engine,
                {
                    fn to_js(self, cx: &mut #path::Context<'cx, E>) -> #path::Result<E::Value<'cx>> {
                        let object = cx.new_object()?;
                        Ok(object.into_value().into_raw())
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
                let readers = fields.named.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap();
                    let key = name.to_string();
                    quote! {
                        #name: object.get_typed(cx, #key)?
                    }
                });
                quote! {
                    impl<'cx, E> #path::FromJs<'cx, E> for #ident
                    where
                        E: #path::Engine,
                    {
                        fn from_js(cx: &mut #path::Context<'cx, E>, value: E::Value<'cx>) -> #path::Result<Self> {
                            let value = #path::Value::new(value);
                            if !value.is_object() {
                                return Err(#path::Error::type_err("expected object"));
                            }
                            let object = value.try_as_object()?;
                            Ok(Self { #( #readers, )* })
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let readers = fields.unnamed.iter().enumerate().map(|(index, _field)| {
                    let index_u32 = index as u32;
                    quote! {
                        object.get_typed(cx, #index_u32)?
                    }
                });
                quote! {
                    impl<'cx, E> #path::FromJs<'cx, E> for #ident
                    where
                        E: #path::Engine,
                    {
                        fn from_js(cx: &mut #path::Context<'cx, E>, value: E::Value<'cx>) -> #path::Result<Self> {
                            let value = #path::Value::new(value);
                            if !value.is_array() {
                                return Err(#path::Error::type_err("expected array"));
                            }
                            let object = value.try_as_object()?;
                            Ok(Self( #( #readers, )* ))
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<'cx, E> #path::FromJs<'cx, E> for #ident
                where
                    E: #path::Engine,
                {
                    fn from_js(_cx: &mut #path::Context<'cx, E>, _value: E::Value<'cx>) -> #path::Result<Self> {
                        Ok(Self)
                    }
                }
            },
        },
        _ => quote!(compile_error!("FromJs can only be derived for structs")),
    }
}
