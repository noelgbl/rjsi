use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Attribute, Data, Expr, Fields, GenericArgument, Lit, Meta, MetaNameValue, PathArguments, Type,
    TypePath,
};

/// Parse rename attribute to get JS field name
pub(crate) fn get_js_field_name(attrs: &[Attribute], rust_name: &str) -> String {
    for attr in attrs {
        if attr.path().is_ident("rename")
            && let Meta::NameValue(MetaNameValue {
                value: Expr::Lit(expr_lit),
                ..
            }) = &attr.meta
            && let Lit::Str(lit_str) = &expr_lit.lit
        {
            return lit_str.value();
        }
    }
    rust_name.to_string()
}

/// Parse js_default attribute to get default value expression
fn get_js_default_value(attrs: &[Attribute]) -> Option<TokenStream2> {
    for attr in attrs {
        if attr.path().is_ident("js_default") {
            // Support both #[js_default] and #[js_default = "value"]
            match &attr.meta {
                Meta::Path(_) => {
                    // #[js_default] - use Default::default()
                    return Some(quote! { Default::default() });
                }
                Meta::NameValue(MetaNameValue {
                    value: Expr::Lit(expr_lit),
                    ..
                }) => {
                    // #[js_default = "value"] - use the literal value
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        let value = lit_str.value();
                        return Some(quote! { #value.into() });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Check if a type is Option<T> and return the inner type T
fn extract_option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(segment) = path.segments.last()
        && segment.ident == "Option"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
    {
        return Some(inner_ty);
    }
    None
}

pub(crate) fn impl_deserialize(input: syn::DeriveInput) -> TokenStream2 {
    let name = input.ident;

    // Get the fields from the struct
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("FromJSValue can only be derived for structs with named fields"),
        },
        _ => panic!("FromJSValue can only be derived for structs"),
    };

    // Generate field extractions
    let field_extractions = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let js_name = get_js_field_name(&field.attrs, &field_name.to_string());
        let js_default_value = get_js_default_value(&field.attrs);

        let js_name_lit = syn::LitStr::new(&js_name, field_name.span());
        let field_name_str = field_name.to_string();

        // Check if field type is Option<T>
        if let Some(_inner_type) = extract_option_inner_type(field_type) {
            // Optional field
            quote! {
                #field_name: match obj.get(#js_name_lit) {
                    Ok(val) => Some(val),
                    Err(e) if e.is_property_not_found() => None,
                    Err(e) => return Err(rjsi::HostError::new(
                        rjsi::error::E_INVALID_ARG,
                        format!("Failed to convert field '{}': {}", #field_name_str, e)
                    ).with_name("TypeError").into()),
                }
            }
        } else if let Some(js_default_expr) = js_default_value {
            // Field with default value
            quote! {
                #field_name: match obj.get(#js_name_lit) {
                    Ok(val) => val,
                    Err(e) if e.is_property_not_found() => #js_default_expr,
                    Err(e) => return Err(rjsi::HostError::new(
                        rjsi::error::E_INVALID_ARG,
                        format!("Failed to convert field '{}': {}", #field_name_str, e)
                    ).with_name("TypeError").into()),
                }
            }
        } else {
            // Required field
            quote! {
                #field_name: obj.get(#js_name_lit).map_err(|e| {
                    if e.is_property_not_found() {
                        rjsi::HostError::new(
                            rjsi::error::E_MISSING_PROPERTY,
                            format!("Required field '{}' is missing", #field_name_str)
                        ).with_name("TypeError")
                    } else {
                        rjsi::HostError::new(
                            rjsi::error::E_INVALID_ARG,
                            format!("Failed to convert field '{}': {}", #field_name_str, e)
                        ).with_name("TypeError")
                    }
                })?
            }
        }
    });

    let expanded = quote! {
        impl rjsi::FromJSValue<rjsi::JSEngineValue> for #name {
            fn from_js_value(ctx: &rjsi::JSContext, value: rjsi::JSValue) -> rjsi::JSResult<Self> {
                let obj = rjsi::JSObject::from_js_value(ctx, value)?;
                Ok(Self {
                    #(#field_extractions,)*
                })
            }
        }

        impl rjsi::function::JSParameterType for #name {}
    };

    expanded
}
