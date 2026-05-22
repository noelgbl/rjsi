use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase
};
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Index, ItemImpl, parse_macro_input};

#[derive(Clone, Copy, Default)]
enum RenameAll {
    #[default]
    None,
    LowerCamelCase,
    UpperCamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
    Lowercase,
    Uppercase,
}

impl RenameAll {
    fn from_lit(s: &str) -> Option<Self> {
        match s {
            "camelCase" => Some(Self::LowerCamelCase),
            "PascalCase" => Some(Self::UpperCamelCase),
            "snake_case" => Some(Self::SnakeCase),
            "SCREAMING_SNAKE_CASE" => Some(Self::ScreamingSnakeCase),
            "kebab-case" => Some(Self::KebabCase),
            "SCREAMING-KEBAB-CASE" => Some(Self::ScreamingKebabCase),
            "lowercase" => Some(Self::Lowercase),
            "UPPERCASE" => Some(Self::Uppercase),
            _ => None,
        }
    }

    fn apply(self, s: &str) -> String {
        match self {
            Self::None => s.to_string(),
            Self::LowerCamelCase => s.to_lower_camel_case(),
            Self::UpperCamelCase => s.to_upper_camel_case(),
            Self::SnakeCase => s.to_snake_case(),
            Self::ScreamingSnakeCase => s.to_shouty_snake_case(),
            Self::KebabCase => s.to_kebab_case(),
            Self::ScreamingKebabCase => s.to_shouty_kebab_case(),
            Self::Lowercase => s.to_lowercase(),
            Self::Uppercase => s.to_uppercase(),
        }
    }
}

fn js_nested_metas(attrs: &[syn::Attribute]) -> impl Iterator<Item = syn::Meta> + use<'_> {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("js"))
        .flat_map(|a| {
            a.meta
                .require_list()
                .and_then(|list| {
                    list.parse_args_with(
                        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                    )
                })
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
        })
}

fn parse_rename_all(attrs: &[syn::Attribute]) -> RenameAll {
    for meta in js_nested_metas(attrs) {
        if let syn::Meta::NameValue(nv) = meta {
            if nv.path.is_ident("rename_all") {
                if let syn::Expr::Lit(el) = &nv.value {
                    if let syn::Lit::Str(s) = &el.lit {
                        if let Some(conv) = RenameAll::from_lit(&s.value()) {
                            return conv;
                        }
                    }
                }
            }
        }
    }
    RenameAll::None
}

fn parse_field_rename(attrs: &[syn::Attribute]) -> Option<String> {
    for meta in js_nested_metas(attrs) {
        if let syn::Meta::NameValue(nv) = meta {
            if nv.path.is_ident("rename") {
                if let syn::Expr::Lit(el) = &nv.value {
                    if let syn::Lit::Str(s) = &el.lit {
                        return Some(s.value());
                    }
                }
            }
        }
    }
    None
}

fn field_key(field_attrs: &[syn::Attribute], rust_name: &str, rename_all: RenameAll) -> String {
    parse_field_rename(field_attrs).unwrap_or_else(|| rename_all.apply(rust_name))
}

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

#[proc_macro_derive(NativeState)]
pub fn derive_native_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let core = core_path();
    quote! {
        impl #impl_generics #core::NativeState for #name #ty_generics #where_clause {}
    }
    .into()
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

#[proc_macro_derive(IntoJs, attributes(js))]
pub fn derive_into_js(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_into_js(&input).into()
}

#[proc_macro_derive(FromJs, attributes(js))]
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
    let rename_all = parse_rename_all(&input.attrs);
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let setters = fields.named.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap();
                    let key = field_key(&field.attrs, &name.to_string(), rename_all);
                    quote! {
                        object.set_typed(cx, #key, self.#name)?;
                    }
                });
                quote! {
                    impl<'cx, E> #path::ToJs<'cx, E> for #ident
                    where
                        E: #path::Engine,
                    {
                        fn to_js(self, cx: &mut #path::Context<'cx, E>) -> #path::Result<#path::Value<'cx, E>> {
                            let object = cx.new_object()?;
                            #( #setters )*
                            Ok(object.into_value())
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
                        fn to_js(self, cx: &mut #path::Context<'cx, E>) -> #path::Result<#path::Value<'cx, E>> {
                            let array_value = cx.eval("[]")?;
                            let array = array_value.try_as_object()?;
                            #( #setters )*
                            Ok(array.into_value())
                        }
                    }
                }
            }
            Fields::Unit => quote! {
                impl<'cx, E> #path::ToJs<'cx, E> for #ident
                where
                    E: #path::Engine,
                {
                    fn to_js(self, cx: &mut #path::Context<'cx, E>) -> #path::Result<#path::Value<'cx, E>> {
                        let object = cx.new_object()?;
                        Ok(object.into_value())
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
    let rename_all = parse_rename_all(&input.attrs);
    let from_js_impl = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let readers = fields.named.iter().map(|field| {
                    let name = field.ident.as_ref().unwrap();
                    let key = field_key(&field.attrs, &name.to_string(), rename_all);
                    quote! {
                        #name: object.get_typed(cx, #key)?
                    }
                });
                quote! {
                    impl<'cx, E> #path::FromJs<'cx, E> for #ident
                    where
                        E: #path::Engine,
                    {
                        fn from_js(cx: &mut #path::Context<'cx, E>, value: #path::Value<'cx, E>) -> #path::Result<Self> {
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
                        fn from_js(cx: &mut #path::Context<'cx, E>, value: #path::Value<'cx, E>) -> #path::Result<Self> {
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
                    fn from_js(_cx: &mut #path::Context<'cx, E>, _value: #path::Value<'cx, E>) -> #path::Result<Self> {
                        Ok(Self)
                    }
                }
            },
        },
        _ => return quote!(compile_error!("FromJs can only be derived for structs")),
    };

    let from_param_impl = quote! {
        impl<'cx, E> #path::FromParam<'cx, E> for #ident
        where
            E: #path::Engine,
        {
            fn param_requirement() -> #path::ParamRequirement {
                #path::ParamRequirement::single()
            }

            fn from_param<'a>(
                params: &mut #path::ParamsAccessor<'a, 'cx, E>,
            ) -> #path::Result<Self> {
                let value = params.arg();
                <Self as #path::FromJs<'cx, E>>::from_js(params.ctx(), value)
            }
        }
    };

    quote! {
        #from_js_impl
        #from_param_impl
    }
}
