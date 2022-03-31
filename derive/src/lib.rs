use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::AttributeArgs;
use syn::Ident;
use syn::Meta;
use syn::NestedMeta;
use syn::{Data, DeriveInput, Field};

mod server;

pub(crate) enum DataStorage {
    Ignored,
    Hashed,
    Stored,
}

pub(crate) struct DerivedRequest {
    idents: Idents,
    data_storage_ty: DataStorage,
}

pub(crate) struct Idents {
    base: Ident,
    request_register: Ident,
    verify_register: Ident,
    request_auth: Ident,
    verify_auth: Ident,
}

pub(crate) struct DeriveData {
    pub(crate) ident: Ident,
    pub(crate) attrs: Vec<Ident>,
    pub(crate) fields: Vec<Field>,
    pub(crate) request: DerivedRequest,
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

        let request_register = Ident::new(&format!("{}RegisterReq", ident), ident.span());
        let verify_register = Ident::new(&format!("{}VerifyRegisterReq", ident), ident.span());
        let request_auth = Ident::new(&format!("{}AuthReq", ident), ident.span());
        let verify_auth = Ident::new(&format!("{}VerifyAuthReq", ident), ident.span());

        let idents = Idents {
            base: attrs
                .get(0)
                .expect("Must provide a Request Data Type!")
                .clone(),
            request_register,
            verify_register,
            request_auth,
            verify_auth,
        };

        let request = DerivedRequest {
            idents,
            data_storage_ty: match attrs.get(1) {
                Some(id) => match id.to_string().as_str() {
                    "Ignored" => DataStorage::Ignored,
                    "Hashed" => DataStorage::Hashed,
                    "Stored" => DataStorage::Stored,
                    _ => unimplemented!(
                        "That period is not supported (support: Ignored, Hashed, Stored)"
                    ),
                },
                None => DataStorage::Stored,
            },
        };

        Self {
            fields,
            attrs,
            ident,
            request,
        }
    }
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn PassServer(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    server::derive((args, input).into()).into()
}
// #[cfg(test)]
// mod test;
