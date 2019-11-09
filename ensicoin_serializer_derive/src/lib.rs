extern crate proc_macro;
use crate::proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(Deserialize)]
pub fn deserialize_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_deserialize_macro(&ast)
}

fn impl_deserialize_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let generics = &ast.generics;

    let mut field_list = quote! {};
    let mut body = quote! {};

    match &ast.data {
        syn::Data::Struct(data) => {
            for field in data.fields.iter() {
                let field_type = &field.ty;
                match &field.ident {
                    Some(field_name) => {
                        body = quote! {
                            #body
                            let #field_name = match <#field_type>::deserialize(de) {
                                Ok(v) => v,
                                Err(e) => {
                                    return Err(ensicoin_serializer::Error::Message(format!(
                                                "Error in reading {} {}: {}",
                                                stringify!(#name),
                                                stringify!(#field_name),
                                                e
                                    )));
                                }
                            };
                        };
                        field_list = quote! {#field_list
                        #field_name,};
                    }
                    None => panic!("Can't derive unamed field in {}", name),
                }
            }
        }
        _ => panic!("Can only derive struts, {} is invalid", name),
    };

    let gen = quote! {
        impl #generics Deserialize for #name #generics {
            fn deserialize(
                de: &mut ensicoin_serializer::Deserializer,
            ) -> ensicoin_serializer::Result<Self> {
                #body
                Ok(#name {#field_list
                })
            }
       }
    };
    gen.into()
}
