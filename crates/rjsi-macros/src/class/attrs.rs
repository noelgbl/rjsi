#![allow(dead_code)]

use heck::ToLowerCamelCase;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, LitStr, Meta, Signature, Token};

#[derive(Clone)]
pub struct JsClassAttrs {
    pub name: Option<String>,
    pub no_constructor: bool,
}

#[derive(Clone, Default)]
pub struct JsMethodsAttrs {
    pub name: Option<String>,
    pub no_constructor: bool,
}

impl Parse for JsClassAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut name = None::<String>;
        let mut no_constructor = false;
        let punct = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for meta in punct {
            match meta {
                Meta::Path(p) if p.is_ident("no_constructor") => {
                    no_constructor = true;
                }
                Meta::NameValue(nv) => {
                    if nv.path.is_ident("name") {
                        if let syn::Expr::Lit(el) = &nv.value {
                            if let syn::Lit::Str(s) = &el.lit {
                                name = Some(s.value());
                            }
                        }
                    }
                }
                Meta::List(list) => {
                    if list.path.is_ident("name") {
                        let nested: LitStr = syn::parse2(list.tokens)?;
                        name = Some(nested.value());
                    }
                }
                _ => {}
            }
        }
        Ok(JsClassAttrs {
            name,
            no_constructor,
        })
    }
}

impl Parse for JsMethodsAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut name = None::<String>;
        let mut no_constructor = false;
        if input.is_empty() {
            return Ok(JsMethodsAttrs::default());
        }
        let punct = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for meta in punct {
            match meta {
                Meta::Path(p) if p.is_ident("no_constructor") => {
                    no_constructor = true;
                }
                Meta::NameValue(nv) if nv.path.is_ident("name") => {
                    if let syn::Expr::Lit(el) = &nv.value {
                        if let syn::Lit::Str(s) = &el.lit {
                            name = Some(s.value());
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(JsMethodsAttrs { name, no_constructor })
    }
}

#[derive(Clone, Debug)]
pub enum MethodKind {
    Constructor,
    Instance,
    Static,
    Getter { prop: String },
    Setter { prop: String },
    Skip,
}

pub fn parse_js_class_on_struct(attrs: &[Attribute]) -> syn::Result<JsClassAttrs> {
    for attr in attrs {
        if attr.path().is_ident("js_class") {
            return attr.parse_args::<JsClassAttrs>();
        }
    }
    Ok(JsClassAttrs {
        name: None,
        no_constructor: false,
    })
}

pub fn parse_js_methods_impl_attrs(attrs: &[Attribute]) -> syn::Result<JsMethodsAttrs> {
    for attr in attrs {
        if attr.path().is_ident("js_methods") {
            return attr.parse_args::<JsMethodsAttrs>();
        }
    }
    Ok(JsMethodsAttrs::default())
}

pub fn classify_method(attrs: &[Attribute], sig: &Signature) -> syn::Result<MethodKind> {
    let mut skip = false;
    let mut is_ctor = false;
    let mut is_static = false;
    let mut is_get = false;
    let mut is_set = false;

    for attr in attrs {
        let id = attr.path().get_ident().map(|i| i.to_string());
        let Some(id) = id else {
            continue;
        };
        match id.as_str() {
            "js_skip" => skip = true,
            "js_constructor" => is_ctor = true,
            "js_static" => is_static = true,
            "js_get" => is_get = true,
            "js_set" => is_set = true,
            _ => {}
        }
    }

    if skip {
        return Ok(MethodKind::Skip);
    }

    if is_ctor {
        return Ok(MethodKind::Constructor);
    }

    let self_info = receiver_style(sig);

    if is_static || self_info.is_none() {
        return Ok(MethodKind::Static);
    }

    if is_get {
        let prop = getter_prop_name(sig)?;
        return Ok(MethodKind::Getter { prop });
    }

    if is_set {
        let prop = setter_prop_name(sig)?;
        return Ok(MethodKind::Setter { prop });
    }

    Ok(MethodKind::Instance)
}

fn receiver_style(sig: &Signature) -> Option<bool> {
    sig.receiver().map(|r| r.mutability.is_some())
}

pub fn js_method_name(rust_ident: &syn::Ident) -> String {
    rust_ident.to_string().to_lower_camel_case()
}

fn getter_prop_name(sig: &Signature) -> syn::Result<String> {
    Ok(sig.ident.to_string())
}

fn setter_prop_name(sig: &Signature) -> syn::Result<String> {
    let s = sig.ident.to_string();
    let prop = s.strip_prefix("set_").unwrap_or(&s).to_string();
    Ok(prop)
}

pub fn strip_js_attrs_from_impl(mut input: syn::ItemImpl) -> syn::ItemImpl {
    for item in &mut input.items {
        if let syn::ImplItem::Fn(f) = item {
            f.attrs.retain(|a| !is_js_bridge_attr(a));
        }
    }
    input.attrs.retain(|a| !a.path().is_ident("js_methods"));
    input
}

fn is_js_bridge_attr(attr: &Attribute) -> bool {
    attr.path()
        .get_ident()
        .map(|i| {
            matches!(
                i.to_string().as_str(),
                "js_constructor" | "js_static" | "js_get" | "js_set" | "js_skip" | "js_symbol"
            ) || i.to_string().starts_with("js_")
        })
        .unwrap_or(false)
}
