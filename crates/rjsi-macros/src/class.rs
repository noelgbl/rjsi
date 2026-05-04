mod attrs;

use attrs::{JsMethodsAttrs, classify_method, js_method_name, MethodKind, strip_js_attrs_from_impl};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{DeriveInput, FnArg, ItemImpl, Pat, ReturnType, Type};

use crate::core_path;


pub fn expand_js_class(input: DeriveInput) -> TokenStream2 {
    let core = core_path();
    let ident = &input.ident;
    let attrs = match attrs::parse_js_class_on_struct(&input.attrs) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };
    let name_lit = attrs
        .name
        .as_ref()
        .map(|s| syn::LitStr::new(s, Span::call_site()))
        .unwrap_or_else(|| syn::LitStr::new(&ident.to_string(), Span::call_site()));
    let constructor_body = if attrs.no_constructor {
        quote! { Err(#core::JsError::type_err("constructor is not exposed")) }
    } else {
        quote! { Err(#core::JsError::type_err("constructor not implemented")) }
    };

    quote! {
        impl<E: #core::Engine> #core::JsClass<E> for #ident {
            const NAME: &'static str = #name_lit;

            fn define_prototype<'cx>(
                _cx: &mut #core::Context<'cx, E>,
                _proto: &#core::Object<'cx, E>,
            ) -> #core::JsResult<'cx, E, ()>
            where
                E: #core::ClassEngine,
            {
                Ok(())
            }

            fn construct<'cx, 'rt>(
                _cx: &mut #core::CallbackCx<'cx, 'rt, E>,
                _args: #core::Args<'rt, E>,
            ) -> #core::JsResult<'rt, E, Self>
            where
                E: #core::ClassEngine,
            {
                #constructor_body
            }
        }
    }
}


pub fn expand_js_methods(attr: TokenStream2, input: ItemImpl) -> TokenStream2 {
    let core = core_path();

    let opts: JsMethodsAttrs = match syn::parse2(attr) {
        Ok(o) => o,
        Err(e) => return e.to_compile_error(),
    };

    let self_ty = input.self_ty.clone();

    let name_str = opts.name.unwrap_or_else(|| {
        if let Type::Path(tp) = self_ty.as_ref() {
            tp.path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_else(|| "NativeClass".to_string())
        } else {
            "NativeClass".to_string()
        }
    });
    let name_lit = syn::LitStr::new(&name_str, Span::call_site());

    let host_ident = host_struct_ident(&self_ty);

    let mut ctor_tokens: Option<TokenStream2> = None;
    let mut host_items: Vec<TokenStream2> = Vec::new();
    let mut method_registrations: Vec<TokenStream2> = Vec::new();

    for item in &input.items {
        let syn::ImplItem::Fn(f) = item else { continue };

        let kind = match classify_method(&f.attrs, &f.sig) {
            Ok(k) => k,
            Err(e) => return e.to_compile_error(),
        };

        match kind {
            MethodKind::Skip => {}

            MethodKind::Constructor => {
                let arg_extractions = constructor_arg_extractions(&core, &f.sig);
                let call_arg_idents = call_idents(&f.sig);
                let fn_name = &f.sig.ident;

                ctor_tokens = Some(quote! {
                    fn construct<'cx, 'rt>(
                        cx: &mut #core::CallbackCx<'cx, 'rt, E>,
                        args: #core::Args<'rt, E>,
                    ) -> #core::JsResult<'rt, E, Self>
                    where
                        E: #core::ClassEngine,
                    {
                        #( #arg_extractions )*
                        Ok(Self::#fn_name( #( #call_arg_idents ),* ))
                    }
                });
            }

            MethodKind::Instance => {
                let rust_name = &f.sig.ident;
                let js_name_lit =
                    syn::LitStr::new(&js_method_name(rust_name), Span::call_site());
                let is_mut = f.sig.receiver().map_or(false, |r| r.mutability.is_some());
                let arg_extractions = method_arg_extractions(&core, &f.sig);
                let call_arg_idents = call_idents(&f.sig);
                let ret_expr = method_return_expr(&core, &f.sig, is_mut, rust_name, &call_arg_idents);

                let host_fn = instance_host_fn_ident(rust_name);
                host_items.push(quote! {
                    #[inline]
                    fn #host_fn<'cx, 'rt, E: #core::ClassEngine>(
                        cb_cx: &mut #core::CallbackCx<'cx, 'rt, E>,
                        this: #core::Value<'rt, E>,
                        args: #core::Args<'rt, E>,
                    ) -> #core::JsResult<'rt, E, #core::Value<'rt, E>> {
                        let _ = args.len();
                        let __this_obj = #core::Object::new(
                            E::value_to_object(this.into_raw())
                                .ok_or_else(|| #core::JsError::type_err("expected object"))?
                        );
                        let ptr = unsafe {
                            E::class_get_instance_ptr::<#self_ty>(cb_cx.cx(), &__this_obj)
                        }
                        .ok_or_else(|| #core::JsError::type_err(
                            concat!("not an instance of ", stringify!(#self_ty))
                        ))?;
                        #( #arg_extractions )*
                        #ret_expr
                    }
                });

                method_registrations.push(quote! {
                    {
                        let __f = cx.function(#js_name_lit, #host_ident::#host_fn)?;
                        proto.set(cx, #js_name_lit, __f.into_value())?;
                    }
                });
            }

            MethodKind::Static => {
                let rust_name = &f.sig.ident;
                let js_name_lit =
                    syn::LitStr::new(&js_method_name(rust_name), Span::call_site());
                let arg_extractions = method_arg_extractions(&core, &f.sig);
                let call_arg_idents = call_idents(&f.sig);
                let ret_expr = static_return_expr(&core, &f.sig, rust_name, &call_arg_idents);

                let host_fn = static_host_fn_ident(rust_name);
                host_items.push(quote! {
                    #[inline]
                    fn #host_fn<'cx, 'rt, E: #core::ClassEngine>(
                        cb_cx: &mut #core::CallbackCx<'cx, 'rt, E>,
                        _this: #core::Value<'rt, E>,
                        args: #core::Args<'rt, E>,
                    ) -> #core::JsResult<'rt, E, #core::Value<'rt, E>> {
                        #( #arg_extractions )*
                        #ret_expr
                    }
                });

                method_registrations.push(quote! {
                    {
                        let __f = cx.function(#js_name_lit, #host_ident::#host_fn)?;
                        proto.set(cx, #js_name_lit, __f.into_value())?;
                    }
                });
            }

            MethodKind::Getter { .. } | MethodKind::Setter { .. } => {}
        }
    }

    let construct_body = ctor_tokens.unwrap_or_else(|| {
        if opts.no_constructor {
            quote! {
                fn construct<'cx, 'rt>(
                    _cx: &mut #core::CallbackCx<'cx, 'rt, E>,
                    _args: #core::Args<'rt, E>,
                ) -> #core::JsResult<'rt, E, Self>
                where
                    E: #core::ClassEngine,
                {
                    Err(#core::JsError::type_err("constructor is not exposed"))
                }
            }
        } else {
            quote! {
                fn construct<'cx, 'rt>(
                    _cx: &mut #core::CallbackCx<'cx, 'rt, E>,
                    _args: #core::Args<'rt, E>,
                ) -> #core::JsResult<'rt, E, Self>
                where
                    E: #core::ClassEngine,
                {
                    Err(#core::JsError::type_err("constructor not implemented"))
                }
            }
        }
    });

    let stripped_input = strip_js_attrs_from_impl(input);

    let host_type = if host_items.is_empty() {
        quote! {}
    } else {
        quote! {
            #[doc(hidden)]
            struct #host_ident;

            #[doc(hidden)]
            impl #host_ident {
                #( #host_items )*
            }
        }
    };

    quote! {
        #stripped_input

        #host_type

        impl<E: #core::ClassEngine> #core::JsClass<E> for #self_ty {
            const NAME: &'static str = #name_lit;

            fn define_prototype<'cx>(
                cx: &mut #core::Context<'cx, E>,
                proto: &#core::Object<'cx, E>,
            ) -> #core::JsResult<'cx, E, ()>
            {
                #( #method_registrations )*
                Ok(())
            }

            #construct_body
        }
    }
}

fn constructor_arg_extractions(core: &TokenStream2, sig: &syn::Signature) -> Vec<TokenStream2> {
    let mut result = Vec::new();
    let mut idx: usize = 0;
    for input in &sig.inputs {
        let FnArg::Typed(pat_ty) = input else { continue };
        let ty = &pat_ty.ty;
        let arg_ident = arg_ident_from_pat(&pat_ty.pat, idx);
        let idx_lit = syn::Index::from(idx);
        result.push(quote! {
            let #arg_ident: #ty = #core::FromJs::from_js(
                cx.cx(),
                args.get(#idx_lit)
                    .ok_or_else(|| #core::JsError::type_err(
                        concat!("missing argument ", stringify!(#idx_lit))
                    ))?
                    .into_raw(),
            )?;
        });
        idx += 1;
    }
    result
}

fn method_arg_extractions(core: &TokenStream2, sig: &syn::Signature) -> Vec<TokenStream2> {
    let mut result = Vec::new();
    let mut idx: usize = 0;
    for input in &sig.inputs {
        match input {
            FnArg::Receiver(_) => continue,
            FnArg::Typed(pat_ty) => {
                let ty = &pat_ty.ty;
                let arg_ident = arg_ident_from_pat(&pat_ty.pat, idx);
                let idx_lit = syn::Index::from(idx);
                result.push(quote! {
                    let #arg_ident: #ty = #core::FromJs::from_js(
                        cb_cx.cx(),
                        args.get(#idx_lit)
                            .ok_or_else(|| #core::JsError::type_err(
                                concat!("missing argument ", stringify!(#idx_lit))
                            ))?
                            .into_raw(),
                    )?;
                });
                idx += 1;
            }
        }
    }
    result
}

fn call_idents(sig: &syn::Signature) -> Vec<syn::Ident> {
    let mut result = Vec::new();
    let mut idx: usize = 0;
    for input in &sig.inputs {
        match input {
            FnArg::Receiver(_) => continue,
            FnArg::Typed(pat_ty) => {
                result.push(arg_ident_from_pat(&pat_ty.pat, idx));
                idx += 1;
            }
        }
    }
    result
}

fn arg_ident_from_pat(pat: &Pat, idx: usize) -> syn::Ident {
    if let Pat::Ident(pi) = pat {
        pi.ident.clone()
    } else {
        syn::Ident::new(&format!("__arg_{idx}"), Span::call_site())
    }
}

fn method_return_expr(
    core: &TokenStream2,
    sig: &syn::Signature,
    is_mut: bool,
    rust_name: &syn::Ident,
    call_arg_idents: &[syn::Ident],
) -> TokenStream2 {
    let self_ref = if is_mut {
        quote! { let instance = unsafe { &mut *ptr }; }
    } else {
        quote! { let instance = unsafe { &*ptr }; }
    };

    match &sig.output {
        ReturnType::Default => quote! {
            #self_ref
            instance.#rust_name( #( #call_arg_idents ),* );
            Ok(cb_cx.cx().undefined())
        },
        ReturnType::Type(_, ty) => {
            if is_result_type(ty) {
                quote! {
                    #self_ref
                    instance.#rust_name( #( #call_arg_idents ),* )
                        .map(|__v| #core::Value::<E>::new(E::function_to_value(__v.into_raw())))
                }
            } else {
                quote! {
                    #self_ref
                    let __result: #ty = instance.#rust_name( #( #call_arg_idents ),* );
                    #core::ToJs::to_js(__result, cb_cx.cx())
                        .map(|__v| #core::Value::<E>::new(__v))
                }
            }
        }
    }
}

fn static_return_expr(
    core: &TokenStream2,
    sig: &syn::Signature,
    rust_name: &syn::Ident,
    call_arg_idents: &[syn::Ident],
) -> TokenStream2 {
    match &sig.output {
        ReturnType::Default => quote! {
            Self::#rust_name( #( #call_arg_idents ),* );
            Ok(cb_cx.cx().undefined())
        },
        ReturnType::Type(_, ty) => quote! {
            let __result: #ty = Self::#rust_name( #( #call_arg_idents ),* );
            #core::ToJs::to_js(__result, cb_cx.cx())
                .map(|__v| #core::Value::<E>::new(__v))
        },
    }
}

fn is_result_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        tp.path
            .segments
            .last()
            .map_or(false, |s| s.ident == "JsResult" || s.ident == "Result")
    } else {
        false
    }
}

fn host_struct_ident(self_ty: &Type) -> syn::Ident {
    let suffix = match self_ty {
        Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_else(|| "Type".to_string()),
        _ => "Type".to_string(),
    };
    syn::Ident::new(
        &format!("__RjsiHost_{}", suffix),
        Span::call_site(),
    )
}

fn instance_host_fn_ident(rust_name: &syn::Ident) -> syn::Ident {
    syn::Ident::new(
        &format!("__rjsi_i_{}", rust_name),
        Span::call_site(),
    )
}

fn static_host_fn_ident(rust_name: &syn::Ident) -> syn::Ident {
    syn::Ident::new(
        &format!("__rjsi_s_{}", rust_name),
        Span::call_site(),
    )
}
