
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse_macro_input, Attribute, AttributeArgs, Ident, Meta, NestedMeta, Data, DeriveInput, Field, Lit};

mod server;
mod tests;

#[derive(Debug)]
pub(crate) enum DataStorage {
    Ignored,
    Hashed,
    Stored,
}

#[derive(Debug)]
pub(crate) struct DerivedRequest {
    idents: Idents,
    data_storage_ty: DataStorage,
}

#[derive(Debug)]
pub(crate) struct Idents {
    base: Ident,
    request_register: Ident,
    verify_register: Ident,
    request_auth: Ident,
    verify_auth: Ident,
}

#[derive(Debug)]
pub(crate) struct DeriveData {
    pub(crate) ident: Ident,
    pub(crate) sub_attrs: Vec<Attribute>,
    pub(crate) fields: Vec<Field>,
    pub(crate) request: DerivedRequest,
    pub(crate) server_ty: NestedMeta,
    pub(crate) ignore_tests: bool,
}

impl From<(Vec<NestedMeta>, TokenStream)> for DeriveData {
    fn from(tokens: (Vec<NestedMeta>, TokenStream)) -> Self {
        let (args, tok) = tokens;
        let DeriveInput {
            data, ident, attrs, ..
        } = syn::parse(tok).unwrap();

        let fields = match data {
            Data::Struct(data) => data.fields.into_iter().collect(),
            _ => unimplemented!("DeriveData is only for a struct"),
        };

        let mut data: Option<Ident> = None;
        let mut store: Option<DataStorage> = None;
        let mut ty: Option<NestedMeta> = None;
        let mut ignore_tests: bool = false;

        args.iter().for_each(|v| match v {
            NestedMeta::Meta(meta) => {
                let name = meta.path().get_ident().unwrap();
                match name.to_string().as_str() {
                    "data" => {
                        let nested = match meta {
                            Meta::List(list) => &list.nested,
                            _ => panic!("data must be a list"),
                        };

                        let first = nested.first();
                        data = first.map(|nested| match nested {
                            NestedMeta::Lit(_) => panic!("data must be an identifier"),
                            NestedMeta::Meta(meta) => meta.path().get_ident().unwrap().clone(),
                        });
                    }
                    "store" => {
                        let nested = match meta {
                            Meta::List(list) => &list.nested,
                            _ => panic!("data must be a list"),
                        };

                        let first = nested.first();
                        store = first.map(|nested| match nested {
                            NestedMeta::Lit(_) => panic!("data must be an identifier"),
                            NestedMeta::Meta(meta) => {
                                match meta.path().get_ident().unwrap().to_string().as_str() {
                                    "Ignored" => DataStorage::Ignored,
                                    "Hashed" => DataStorage::Hashed,
                                    "Stored" => DataStorage::Stored,
                                    _ => unimplemented!("That period is not supported (support: Ignored, Hashed, Stored)"),
                                }
                            }
                        });
                    }
                    "ty" => {
                        let nested = match meta {
                            Meta::List(list) => &list.nested,
                            _ => panic!("data must be a list"),
                        };

                        let first = nested.first();
                        ty = first.map(|nested| match nested {
                            NestedMeta::Lit(_) => panic!("data must be an identifier"),
                            NestedMeta::Meta(_) => nested.clone() 
                        });
                    },
                    "ignore_tests" => {
                        let nested = match meta {
                            Meta::List(list) => &list.nested,
                            _ => panic!("data must be a list"),
                        };

                        let first = nested.first();
                        ignore_tests = first.map(|f| {
                            match f {
                                NestedMeta::Lit(bool) => {
                                    match bool {
                                        Lit::Bool(v) => v.value,
                                        _ => panic!("data must be a bool"),
                                    }
                                },
                                _ => panic!("data must be a bool literal"),
                            }
                        }).unwrap_or_default();
                    }
                    _ => unimplemented!(),
                }
            }
            _ => unimplemented!(),
        });

        let request_register = Ident::new(&format!("{}RegisterReq", ident), ident.span());
        let verify_register = Ident::new(&format!("{}VerifyRegisterReq", ident), ident.span());
        let request_auth = Ident::new(&format!("{}AuthReq", ident), ident.span());
        let verify_auth = Ident::new(&format!("{}VerifyAuthReq", ident), ident.span());

        let idents = Idents {
            base: data.expect("Must provide a Request Data Type!"),
            request_register,
            verify_register,
            request_auth,
            verify_auth,
        };

        let request = DerivedRequest {
            idents,
            data_storage_ty: store.expect("Must provide a Data Storage Type!"),
        };

        Self {
            fields,
            sub_attrs: attrs,
            ident,
            request,
            server_ty: ty.expect("Must provide a Server Type!"),
            ignore_tests
        }
    }
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn PassServer(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let derived = (args, input).into();
    let server = server::derive(&derived);

    let tests = if derived.ignore_tests {
        quote::quote!{}
    } else {
        tests::derive(&derived)
    };

    let re_struct: TokenStream2 = derived.into();

    quote::quote! {
        #re_struct

        #server

        #tests
    }
    .into()
}
