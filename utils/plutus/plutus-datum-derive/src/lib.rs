//! This crate creates the attribute macro `ToDatum`

use proc_macro::TokenStream;
use quote::quote;

extern crate alloc;

use alloc::vec::Vec;
use syn::{GenericParam, TypeParamBound};

/// Derives ToDatum instance for annotated struct.
/// `constructor_datum` parameter should be used on a single field structs to decide if the field
/// should be mapped to datum directly or wrapped in constructor datum with one variant and single
/// field. Such distinction is required because exactly same Datum encoding, as in
/// <https://github.com/input-output-hk/partner-chains-smart-contracts>, is required.
#[proc_macro_derive(ToDatum, attributes(constructor_datum))]
pub fn to_datum_derive(input: TokenStream) -> TokenStream {
	let ast = syn::parse(input).expect("Cannot parse source");

	impl_to_datum_derive(&ast)
}

fn impl_to_datum_derive(ast: &syn::DeriveInput) -> TokenStream {
	let name = &ast.ident;
	let data = &ast.data;
	let generics = &ast.generics;
	let mut bounded_generics = generics.clone();
	for generic_param in bounded_generics.params.iter_mut() {
		if let GenericParam::Type(ref mut type_param) = *generic_param {
			type_param.bounds.push(TypeParamBound::Trait(syn::parse_quote!(ToDatum)));
		}
	}
	let has_constructor_datum_attribute = ast.attrs.iter().any(|attr| {
		attr.path()
			.segments
			.first()
			.filter(|ps| ps.ident == *"constructor_datum")
			.is_some()
	});
	let body = match data {
		syn::Data::Struct(ds) => match &ds.fields {
			syn::Fields::Unnamed(fields_unnamed) => {
				let len = fields_unnamed.unnamed.len();
				if len == 1 && !has_constructor_datum_attribute {
					quote! { self.0.to_datum() }
				} else {
					let fields: Vec<_> = core::ops::Range { start: 0, end: len }
						.map(syn::Index::from)
						.map(|i| quote! { self.#i.to_datum()})
						.collect();
					quote! {
						Datum::ConstructorDatum { constructor: 0, fields: vec![#(#fields),*] }
					}
				}
			},
			syn::Fields::Named(fields_named) => {
				let idents: Vec<_> =
					fields_named.named.iter().filter_map(|f| f.ident.clone()).collect();
				match (&idents[..], has_constructor_datum_attribute) {
					([ident], false) => quote! { self.#ident.to_datum() },
					(idents, _) => {
						let fields: Vec<_> =
							idents.iter().map(|token| quote! { self.#token.to_datum()}).collect();
						quote! {
							Datum::ConstructorDatum { constructor: 0, fields: vec![#(#fields),*] }
						}
					},
				}
			},
			syn::Fields::Unit => quote! {
				Datum::ConstructorDatum { constructor: 0, fields: Vec::new() }
			},
		},
		syn::Data::Enum(_de) => quote! {
			compile_error!("ToDatum isn't yet implemented for Enum types"),
		},
		syn::Data::Union(_du) => quote! {
			compile_error!("ToDatum isn't yet implemented for Union types"),
		},
	};
	let gen = quote! {
		impl #bounded_generics ToDatum for #name #generics {
			fn to_datum(&self) -> Datum {
				#body
			}
		}
	};
	gen.into()
}
