use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, Fields};

use crate::deserialize::get_js_field_name;

pub(crate) fn impl_serialize(input: syn::DeriveInput) -> TokenStream2 {
    let name = input.ident;

    // Get the fields from the struct
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("IntoJSObj can only be derived for structs with named fields"),
        },
        _ => panic!("IntoJSObj can only be derived for structs"),
    };

    // Generate field assignments
    let field_assignments = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let js_name = get_js_field_name(&field.attrs, &field_name.to_string());

        let js_name_lit = syn::LitStr::new(&js_name, field_name.span());

        // Check if field type is Option<T>
        let is_option = if let syn::Type::Path(type_path) = field_type {
            type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident == "Option")
                .unwrap_or(false)
        } else {
            false
        };

        if is_option {
            // For Option<T>, only set the property if Some(value)
            quote! {
                if let Some(ref value) = self.#field_name {
                    obj.set(#js_name_lit, value.clone())?;
                }
            }
        } else {
            // For non-optional fields, always set the property
            quote! {
                obj.set(#js_name_lit, self.#field_name.clone())?;
            }
        }
    });

    let expanded = quote! {
        impl rjsi::IntoJSValue<rjsi::JSEngineValue> for #name {
            fn into_js_value(self, ctx: &rjsi::JSContext) -> rjsi::JSValue {
                let obj = rjsi::JSObject::new(ctx);

                // Set each field on the object
                let result: rjsi::JSResult<()> = (|| {
                    #(#field_assignments)*
                    Ok(())
                })();

                // If setting properties failed, return undefined
                match result {
                    Ok(()) => obj.into_js_value(),
                    Err(_) => rjsi::JSValue::undefined(ctx),
                }
            }
        }

        impl rjsi::function::JSParameterType for #name {}
    };

    expanded
}
