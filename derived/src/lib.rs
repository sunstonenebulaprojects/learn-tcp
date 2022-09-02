use std::str::FromStr;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Fields, Ident};

fn require_data_struct(item: &Data) -> syn::Result<&DataStruct> {
    match item {
        Data::Struct(s) => Ok(s),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "Only struct is supported",
        )),
    }
}

fn require_data_enum(item: &Data) -> syn::Result<&DataEnum> {
    match item {
        Data::Enum(e) => Ok(e),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "Only enum is supported",
        )),
    }
}

fn require_named_fields(item: &DataStruct) -> syn::Result<&syn::FieldsNamed> {
    match item.fields {
        Fields::Named(ref fields) => Ok(fields),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "Only named fields are supported",
        )),
    }
}

fn generate_output(name: &Ident, input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let data_struct = require_data_struct(&input.data)?;
    let named_fields = require_named_fields(&data_struct)?;

    let methods = named_fields
        .named
        .iter()
        .filter_map(|f| f.ident.as_ref().and_then(|ident| Some((ident, &f.ty))))
        .map(|(ident, ty)| {
            let setter = format_ident!("set_{}", ident);
            let macro_attr = "#[instrument(skip_all)]";
            let instrument = proc_macro2::TokenStream::from_str(macro_attr).unwrap();
            quote! {
                #instrument
                pub fn #setter (&mut self, val: #ty) -> &mut Self {
                    info!(old = self.#ident, new = val, concat!("Changed ", stringify!(#ident)));
                    self.#ident = val;
                    self
                }
                pub fn #ident (&self) -> #ty {
                    self.#ident
                }
            }
        });

    Ok(quote! {
        impl #name {
            #(#methods)*
        }
    })
}

/// Generate setter and getter for each struct field
/// impl <StructName> {
///     pub fn set_<field_name> (&mut self, val: <field_type>) -> &mut Self {
///         self.<field_name> = val;
///         self
///     }
///     pub fn <field_name> (&self) -> <field_type> {
///         self.<field_name>
///     }
/// }
#[proc_macro_derive(Accessors)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let output = match generate_output(name, &input) {
        Ok(output) => {
            quote!(#output)
        }
        Err(error) => {
            let error = error.to_compile_error();
            quote! {
                #error
            }
        }
    };
    TokenStream::from(output)
}

fn generate_handle_events_output(
    name: &Ident,
    input: &DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let data_enum = require_data_enum(&input.data)?;

    let mut on_segment = Vec::new();
    let mut passive_open = Vec::new();
    let mut open = Vec::new();
    let mut close = Vec::new();
    let mut send = Vec::new();
    for variant in data_enum.variants.clone().into_iter() {
        let ident = &variant.ident;
        on_segment.push(quote!(
            #name::#ident(state) => state.on_segment(iph, tcph, data).await
        ));
        passive_open.push(quote!(
            #name::#ident(state) => state.passive_open().await
        ));
        open.push(quote!(
            #name::#ident(state) => state.open(quad).await
        ));
        close.push(quote!(
            #name::#ident(state) => state.close(quad).await
        ));
        send.push(quote!(
            #name::#ident(state) => state.send(quad, data).await
        ));
    }

    let macro_attr = "#[async_trait]";
    let x = proc_macro2::TokenStream::from_str(macro_attr).unwrap();

    Ok(quote! {
        #x
        impl HandleEvents for #name {
            async fn on_segment(
                &self,
                iph: etherparse::Ipv4Header,
                tcph: etherparse::TcpHeader,
                data: Vec<u8>
            ) -> TrustResult<Option<TransitionState>> {
                match self {
                    #(#on_segment),*
                }
            }

            async fn passive_open(&self) -> TrustResult<Option<TransitionState>> {
                match self {
                    #(#passive_open),*
                }
            }

            async fn open(&self, quad: Quad) -> TrustResult<Option<TransitionState>> {
                match self {
                    #(#open),*
                }
            }

            async fn close(&self, quad: Quad) -> TrustResult<Option<TransitionState>> {
                match self {
                    #(#close),*
                }
            }

            async fn send(
                &self,
                quad: Quad,
                data: Vec<u8>
            ) -> TrustResult<Option<TransitionState>> {
                match self {
                    #(#send),*
                }
            }
        }
    }
    .into())
}

/// Generate delegation calls for each enum variant for each HandleEvents trait method
/// For example:
/// async fn send(
///     &self,
///     quad: Quad,
///     data: Vec<u8>
/// ) -> TrustResult<Option<TransitionState>> {
///         match self {
///             State::Closed(state) => state.send(state.close(quad, data)).await,
///             State::Listen(state) => state.send(state.close(quad, data)).await,
///             State::SynSent(state) => state.send(state.close(quad, data)).await,
///             State::SynRcvd(state) => state.send(state.close(quad, data)).await,
///             State::Estab(state) => state.send(state.close(quad, data)).await,
///             State::LastAck(state) => state.send(state.close(quad, data)).await,
///             State::AckRcvd(state) => state.send(state.close(quad, data)).await,
///         }
/// }
#[proc_macro_derive(HandleEvents)]
pub fn handle_events(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let output = match generate_handle_events_output(name, &input) {
        Ok(output) => {
            quote!(#output)
        }
        Err(error) => {
            let error = error.to_compile_error();
            quote! {
                #error
            }
        }
    };
    TokenStream::from(output)
}
