use proc_macro::TokenStream;

fn impl_component_trait(ast: syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    quote::quote! {
        impl sparse_ecs::world::Component for #name {}
    }
    .into()
}

#[proc_macro_derive(Component)]
pub fn component_derive_macro(item: TokenStream) -> TokenStream {
    let ast = syn::parse(item).unwrap();
    impl_component_trait(ast)
}

fn impl_resource_trait(ast: syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    quote::quote! {
        impl sparse_ecs::resource::Resource for #name {}
    }
    .into()
}

#[proc_macro_derive(Resource)]
pub fn resource_derive_macro(item: TokenStream) -> TokenStream {
    let ast = syn::parse(item).unwrap();
    impl_resource_trait(ast)
}
