#![feature(proc_macro_span)]

use syn::{Ident, DeriveInput, LitStr};
use syn::parse::{ParseBuffer};
use syn::Token;
use quote::{quote, ToTokens};
use std::path::PathBuf;
use proc_macro::{Span, TokenStream};
use std::collections::HashMap;
use xml::EventReader;
use xml::reader::XmlEvent;
use std::fs::File;
use syn::token::Token;
use syn::export::TokenStream2;

struct Args(Ident, LitStr);

impl syn::parse::Parse for Args {
	fn parse<'a>(input: &'a ParseBuffer<'a>) -> Result<Self, syn::Error> {
		let type1 = input.parse()?;
		input.parse::<Token![,]>()?;
		let type2 = input.parse()?;
		Ok(Args(type1, type2))
	}
}

#[proc_macro]
pub fn include_glade(args: TokenStream) -> TokenStream {
	let args: Args = syn::parse(args).unwrap();
	let span = args.0.span();
	let module = args.0;
	let include = args.1.value();
	let mut path = span.unwrap().source_file().path().parent().unwrap().to_owned();
	path.push(include);

	let file = File::open(&path).unwrap();

	let parser = EventReader::new(file);
	let mut map = HashMap::new();
	for e in parser {
		match e {
			Ok(XmlEvent::StartElement { name, attributes, namespace }) => {
				if &name.local_name == "object" {
					let id = attributes.iter().find(| attr | attr.name.local_name == "id");
					if id.is_some() {
						let class = attributes.iter().find(| attr | attr.name.local_name == "class");
						if class.is_some() {
							let class_name = class.unwrap().value.to_owned();
							let class_ident = syn::Ident::new(&class_name.replace("Gtk", ""), span);
							map.insert(id.unwrap().value.to_owned(), class_ident);
						}
					}
				}
			}
			Err(e) => {
				println!("Error: {}", e);
				break;
			}
			_ => {}
		}
	}

	let file_data = std::fs::read_to_string(path).unwrap();

	let mut objects = TokenStream2::new();

	for (id, class_ident) in map.iter() {
		let id_ident = syn::Ident::new(id, span);
		objects.extend::<TokenStream2>(quote!{
			mod #id_ident {
				fn get() -> gtk::#class_ident {
					super::get_builder().get_object(&stringify!(#id)).unwrap()
				}
			}
		}.into());
	};

	let objects = Objects(objects);

	let mut header = quote!{
		mod #module {
			static BUILDER: Option<gtk::Builder> = None;
			fn get_builder() -> gtk::Builder {
				if BUILDER.is_none() {
					unsafe {
						let builder_ptr = unsafe {
                            std::mem::transmute::<&Option<gtk::Builder>, &mut Option<gtk::Builder>>(&BUILDER)
						};
						builder_ptr.replace(gtk::Builder::from_string(&stringify!(#file_data)))
					}
				}
				BUILDER.unwrap()
			}
			#objects
		}
	};
	header.into()
}

struct Objects(TokenStream2);

impl ToTokens for Objects {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		tokens.extend(self.0.clone());
	}
}