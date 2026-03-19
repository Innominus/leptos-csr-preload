use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Error, FnArg, Ident, ImplItem, ItemFn, ItemImpl, LitStr, Result, Token,
};

struct LazyArgs {
    name: Option<Ident>,
    preload_paths: Vec<LitStr>,
    preload: bool,
}

impl Parse for LazyArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut preload_paths = Vec::new();
        let mut preload = false;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            match key.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    name = Some(input.parse()?);
                }
                "preload_path" => {
                    input.parse::<Token![=]>()?;
                    preload_paths.push(input.parse()?);
                }
                "preload_paths" => {
                    input.parse::<Token![=]>()?;
                    preload_paths.extend(parse_string_list(input)?);
                }
                "preload" => preload = true,
                _ => {
                    return Err(Error::new(
                        key.span(),
                        "supported arguments: `name = ...`, `preload`, `preload_path = \"...\"`, `preload_paths = [\"...\", ...]`",
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            name,
            preload_paths,
            preload,
        })
    }
}

struct LazyRouteArgs {
    name: Option<Ident>,
    preload_paths: Vec<LitStr>,
}

impl Parse for LazyRouteArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut preload_paths = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "name" => name = Some(input.parse()?),
                "preload_path" => preload_paths.push(input.parse()?),
                "preload_paths" => preload_paths.extend(parse_string_list(input)?),
                _ => {
                    return Err(Error::new(
                        key.span(),
                        "supported arguments: `name = ...`, `preload_path = \"...\"`, `preload_paths = [\"...\", ...]`",
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            name,
            preload_paths,
        })
    }
}

#[proc_macro_attribute]
pub fn lazy(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as LazyArgs);
    let input = parse_macro_input!(input as ItemFn);

    expand_lazy(args, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn lazy_route(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as LazyRouteArgs);
    let input = parse_macro_input!(input as ItemImpl);

    expand_lazy_route(args, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_lazy(args: LazyArgs, input: ItemFn) -> Result<TokenStream2> {
    if args.preload && !args.preload_paths.is_empty() {
        return Err(Error::new_spanned(
            &input.sig.ident,
            "`preload` is mutually exclusive with `preload_path` and `preload_paths`",
        ));
    }

    let fn_name = input.sig.ident.clone();
    let (lazy_attr, source_name, split_prefix) = if let Some(name) = args.name {
        (
            quote!(#[::leptos::lazy(#name)]),
            quote!(stringify!(#name)),
            quote!(concat!(stringify!(#name), "_")),
        )
    } else {
        (
            quote!(#[::leptos::lazy]),
            quote!(stringify!(#fn_name)),
            quote!(concat!(stringify!(#fn_name), "_")),
        )
    };

    let preload_registration = registration_tokens(
        &args.preload_paths,
        args.preload,
        &source_name,
        &split_prefix,
    );

    Ok(quote! {
        #preload_registration
        #lazy_attr
        #input
    })
}

fn expand_lazy_route(args: LazyRouteArgs, mut input: ItemImpl) -> Result<TokenStream2> {
    let route_name = match args.name {
        Some(name) => name,
        None => derive_route_name(&input)?,
    };
    let helper_ident = format_ident!("__leptos_csr_preload_{}_view", route_name);
    let preload_ident = format_ident!("__preload_{}", helper_ident);
    let self_ty = input.self_ty.clone();

    if input
        .items
        .iter()
        .any(|item| matches!(item, ImplItem::Fn(function) if function.sig.ident == "preload"))
    {
        return Err(Error::new_spanned(
            &input,
            "`preload` should not be implemented manually on #[lazy_route] impls",
        ));
    }

    let Some(view_fn) = input.items.iter_mut().find_map(|item| match item {
        ImplItem::Fn(function) if function.sig.ident == "view" => Some(function),
        _ => None,
    }) else {
        return Err(Error::new_spanned(
            &input,
            "missing `view` method on LazyRoute impl",
        ));
    };

    if let Some(asyncness) = view_fn.sig.asyncness {
        return Err(Error::new(
            asyncness.span,
            "`view` method should not be async",
        ));
    }

    let Some(FnArg::Typed(first_arg)) = view_fn.sig.inputs.first().cloned() else {
        return Err(Error::new_spanned(
            &view_fn.sig,
            "`view` must take a `this: Self` argument",
        ));
    };

    let first_arg_pat = first_arg.pat;
    let body = view_fn.block.clone();
    view_fn.sig.asyncness = Some(Default::default());
    view_fn.block = syn::parse2(quote!({ #helper_ident(#first_arg_pat).await }))?;
    input.items.push(syn::parse2(quote! {
        async fn preload() {
            #[cfg(target_arch = "wasm32")]
            #preload_ident().await;
        }
    })?);

    let route_registration = registration_tokens(
        &args.preload_paths,
        false,
        &quote!(stringify!(#route_name)),
        &quote!(concat!(stringify!(#route_name), "_")),
    );

    Ok(quote! {
        #route_registration

        #[::leptos::lazy(#route_name)]
        fn #helper_ident(#first_arg_pat: #self_ty) -> ::leptos::prelude::AnyView {
            #body
        }

        #input
    })
}

fn derive_route_name(input: &ItemImpl) -> Result<Ident> {
    let syn::Type::Path(type_path) = &*input.self_ty else {
        return Err(Error::new_spanned(
            &input.self_ty,
            "only path types are supported for #[lazy_route]",
        ));
    };

    let Some(segment) = type_path.path.segments.last() else {
        return Err(Error::new_spanned(
            &input.self_ty,
            "route type must have a final path segment",
        ));
    };

    Ok(format_ident!(
        "{}",
        segment.ident.to_string().to_case(Case::Snake)
    ))
}

fn parse_string_list(input: ParseStream) -> Result<Vec<LitStr>> {
    let content;
    bracketed!(content in input);
    let items = Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?;
    Ok(items.into_iter().collect())
}

fn registration_tokens(
    preload_paths: &[LitStr],
    preload_globally: bool,
    source_name: &TokenStream2,
    split_prefix: &TokenStream2,
) -> TokenStream2 {
    if preload_paths.is_empty() && !preload_globally {
        return TokenStream2::new();
    }

    let registrations = preload_paths.iter().map(|preload_path| {
        quote! {
            #[cfg(feature = "preload-registry")]
            ::leptos_csr_preload::__private::inventory::submit! {
                ::leptos_csr_preload::RegisteredPreload {
                    source_name: #source_name,
                    preload_path: Some(#preload_path),
                    split_prefix: #split_prefix,
                }
            }
        }
    });

    let global_registration = preload_globally.then(|| {
        quote! {
            #[cfg(feature = "preload-registry")]
            ::leptos_csr_preload::__private::inventory::submit! {
                ::leptos_csr_preload::RegisteredPreload {
                    source_name: #source_name,
                    preload_path: None,
                    split_prefix: #split_prefix,
                }
            }
        }
    });

    quote!(#global_registration #(#registrations)*)
}
