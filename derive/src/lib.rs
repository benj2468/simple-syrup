use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse_macro_input;
use syn::AttributeArgs;
use syn::Meta;
use syn::NestedMeta;
use syn::{Data, DeriveInput, Field};

mod server;

pub(crate) struct DeriveData {
    pub(crate) ident: Ident,
    pub(crate) req_ident: Ident,
    pub(crate) fields: Vec<Field>,
}

impl From<(Vec<NestedMeta>, TokenStream)> for DeriveData {
    fn from(tokens: (Vec<NestedMeta>, TokenStream)) -> Self {
        let (args, tok) = tokens;
        let DeriveInput { data, ident, .. } = syn::parse(tok).unwrap();

        let fields: Vec<Field> = match data {
            Data::Struct(data) => data.fields.into_iter().collect(),
            _ => unimplemented!("DeriveData is only for a struct"),
        };

        let req_ident = match args
            .get(0)
            .expect("First argument must be the request structure")
        {
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
        };

        Self {
            fields,
            req_ident,
            ident,
        }
    }
}

impl From<DeriveData> for TokenStream2 {
    fn from(data: DeriveData) -> Self {
        let DeriveData { ident, fields, .. } = data;
        quote! {
            pub struct #ident {
                pub pool: sqlx::Pool<sqlx::Postgres>,
                #(#fields,)*
            }
        }
    }
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn AuthServer(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    server::derive((args, input).into()).into()
}

// #[cfg(test)]
// mod test;
