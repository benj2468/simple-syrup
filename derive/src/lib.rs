use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::AttributeArgs;
use syn::Ident;
use syn::Meta;
use syn::NestedMeta;
use syn::{Data, DeriveInput, Field};

mod server;

pub(crate) struct Idents {
    request_register: Ident,
    verify_register: Ident,
    request_auth: Ident,
    verify_auth: Ident,
}

impl From<&Ident> for Idents {
    fn from(req_ident: &Ident) -> Self {
        let request_register = Ident::new(&format!("{}RegisterReq", req_ident), req_ident.span());
        let verify_register =
            Ident::new(&format!("{}VerifyRegisterReq", req_ident), req_ident.span());
        let request_auth = Ident::new(&format!("{}AuthReq", req_ident), req_ident.span());
        let verify_auth = Ident::new(&format!("{}VerifyAuthReq", req_ident), req_ident.span());

        Self {
            request_register,
            verify_register,
            request_auth,
            verify_auth,
        }
    }
}

pub(crate) struct DeriveData {
    pub(crate) ident: Ident,
    pub(crate) attrs: Vec<Ident>,
    pub(crate) fields: Vec<Field>,
}

impl From<(Vec<NestedMeta>, TokenStream)> for DeriveData {
    fn from(tokens: (Vec<NestedMeta>, TokenStream)) -> Self {
        let (args, tok) = tokens;
        let DeriveInput { data, ident, .. } = syn::parse(tok).unwrap();

        let fields = match data {
            Data::Struct(data) => data.fields.into_iter().collect(),
            _ => unimplemented!("DeriveData is only for a struct"),
        };

        let attrs: Vec<_> = args
            .iter()
            .map(|v| match v {
                NestedMeta::Meta(m) => match m {
                    Meta::Path(p) => {
                        let seg = p
                            .segments
                            .iter()
                            .next()
                            .expect("Must have at least one segment");
                        seg.ident.clone()
                    }
                    Meta::List(_) => unimplemented!(),
                    Meta::NameValue(_) => unimplemented!(),
                },
                _ => unimplemented!(),
            })
            .collect();

        Self {
            fields,
            attrs,
            ident,
        }
    }
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn PassServer(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    server::derive((args, input).into()).into()
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn PassRequest(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    server::derive_req((args, input).into()).into()
}

// #[cfg(test)]
// mod test;
