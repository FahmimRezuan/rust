//! Converts the Rust AST to the rustdoc document model

use syntax::ast;
use doc::ItemUtils;

export from_srv, extract, to_str, interner;


/* can't import macros yet, so this is copied from token.rs. See its comment
 * there. */
macro_rules! interner_key (
    () => (unsafe::transmute::<(uint, uint),
           &fn(+@@syntax::parse::token::ident_interner)>((-3 as uint, 0u)))
)

// Hack; rather than thread an interner through everywhere, rely on
// thread-local data
fn to_str(id: ast::ident) -> ~str {
    let intr = unsafe{ task::local_data_get(interner_key!()) };

    return *(*intr.get()).get(id);
}

fn interner() -> syntax::parse::token::ident_interner {
    return *(unsafe{ task::local_data_get(interner_key!()) }).get();
}

fn from_srv(
    srv: astsrv::Srv,
    default_name: ~str
) -> doc::Doc {

    //! Use the AST service to create a document tree

    do astsrv::exec(srv) |ctxt| {
        extract(ctxt.ast, default_name)
    }
}

fn extract(
    crate: @ast::crate,
    default_name: ~str
) -> doc::Doc {
    doc::Doc_({
        pages: ~[
            doc::CratePage({
                topmod: top_ModDoc_from_crate(crate, default_name),
            })
        ]
    })
}

fn top_ModDoc_from_crate(
    crate: @ast::crate,
    default_name: ~str
) -> doc::ModDoc {
    ModDoc_from_mod(mk_ItemDoc(ast::crate_node_id, default_name),
                    crate.node.module)
}

fn mk_ItemDoc(id: ast::node_id, name: ~str) -> doc::ItemDoc {
    {
        id: id,
        name: name,
        path: ~[],
        brief: None,
        desc: None,
        sections: ~[],
        reexport: false
    }
}

fn ModDoc_from_mod(
    ItemDoc: doc::ItemDoc,
    module_: ast::_mod
) -> doc::ModDoc {
    doc::ModDoc_({
        item: ItemDoc,
        items: do vec::filter_map(module_.items) |item| {
            let ItemDoc = mk_ItemDoc(item.id, to_str(item.ident));
            match item.node {
              ast::item_mod(m) => {
                Some(doc::ModTag(
                    ModDoc_from_mod(ItemDoc, m)
                ))
              }
              ast::item_foreign_mod(nm) => {
                Some(doc::NmodTag(
                    nModDoc_from_mod(ItemDoc, nm)
                ))
              }
              ast::item_fn(*) => {
                Some(doc::FnTag(
                    FnDoc_from_fn(ItemDoc)
                ))
              }
              ast::item_const(_, _) => {
                Some(doc::ConstTag(
                    ConstDoc_from_const(ItemDoc)
                ))
              }
              ast::item_enum(enum_definition, _) => {
                Some(doc::EnumTag(
                    EnumDoc_from_enum(ItemDoc, enum_definition.variants)
                ))
              }
              ast::item_trait(_, _, methods) => {
                Some(doc::TraitTag(
                    TraitDoc_from_trait(ItemDoc, methods)
                ))
              }
              ast::item_impl(_, _, _, methods) => {
                Some(doc::ImplTag(
                    ImplDoc_from_impl(ItemDoc, methods)
                ))
              }
              ast::item_ty(_, _) => {
                Some(doc::TyTag(
                    TyDoc_from_ty(ItemDoc)
                ))
              }
              _ => None
            }
        },
        index: None
    })
}

fn nModDoc_from_mod(
    ItemDoc: doc::ItemDoc,
    module_: ast::foreign_mod
) -> doc::NmodDoc {
    let mut fns = ~[];
    for module_.items.each |item| {
        let ItemDoc = mk_ItemDoc(item.id, to_str(item.ident));
        match item.node {
          ast::foreign_item_fn(*) => {
            vec::push(fns, FnDoc_from_fn(ItemDoc));
          }
          ast::foreign_item_const(*) => {} // XXX: Not implemented.
        }
    }
    {
        item: ItemDoc,
        fns: fns,
        index: None
    }
}

fn FnDoc_from_fn(ItemDoc: doc::ItemDoc) -> doc::FnDoc {
    {
        item: ItemDoc,
        sig: None
    }
}

fn ConstDoc_from_const(ItemDoc: doc::ItemDoc) -> doc::ConstDoc {
    {
        item: ItemDoc,
        sig: None
    }
}

#[test]
fn should_extract_const_name_and_id() {
    let doc = test::mk_doc(~"const a: int = 0;");
    assert doc.cratemod().consts()[0].id() != 0;
    assert doc.cratemod().consts()[0].name() == ~"a";
}

fn EnumDoc_from_enum(
    ItemDoc: doc::ItemDoc,
    variants: ~[ast::variant]
) -> doc::EnumDoc {
    {
        item: ItemDoc,
        variants: variantdocs_from_variants(variants)
    }
}

fn variantdocs_from_variants(
    variants: ~[ast::variant]
) -> ~[doc::VariantDoc] {
    vec::map(variants, variantdoc_from_variant)
}

fn variantdoc_from_variant(variant: ast::variant) -> doc::VariantDoc {

    {
        name: to_str(variant.node.name),
        desc: None,
        sig: None
    }
}

#[test]
fn should_extract_enums() {
    let doc = test::mk_doc(~"enum e { v }");
    assert doc.cratemod().enums()[0].id() != 0;
    assert doc.cratemod().enums()[0].name() == ~"e";
}

#[test]
fn should_extract_enum_variants() {
    let doc = test::mk_doc(~"enum e { v }");
    assert doc.cratemod().enums()[0].variants[0].name == ~"v";
}

fn TraitDoc_from_trait(
    ItemDoc: doc::ItemDoc,
    methods: ~[ast::trait_method]
) -> doc::TraitDoc {
    {
        item: ItemDoc,
        methods: do vec::map(methods) |method| {
            match method {
              ast::required(ty_m) => {
                {
                    name: to_str(ty_m.ident),
                    brief: None,
                    desc: None,
                    sections: ~[],
                    sig: None,
                    implementation: doc::Required,
                }
              }
              ast::provided(m) => {
                {
                    name: to_str(m.ident),
                    brief: None,
                    desc: None,
                    sections: ~[],
                    sig: None,
                    implementation: doc::Provided,
                }
              }
            }
        }
    }
}

#[test]
fn should_extract_traits() {
    let doc = test::mk_doc(~"trait i { fn f(); }");
    assert doc.cratemod().traits()[0].name() == ~"i";
}

#[test]
fn should_extract_trait_methods() {
    let doc = test::mk_doc(~"trait i { fn f(); }");
    assert doc.cratemod().traits()[0].methods[0].name == ~"f";
}

fn ImplDoc_from_impl(
    ItemDoc: doc::ItemDoc,
    methods: ~[@ast::method]
) -> doc::ImplDoc {
    {
        item: ItemDoc,
        trait_types: ~[],
        self_ty: None,
        methods: do vec::map(methods) |method| {
            {
                name: to_str(method.ident),
                brief: None,
                desc: None,
                sections: ~[],
                sig: None,
                implementation: doc::Provided,
            }
        }
    }
}

#[test]
fn should_extract_impl_methods() {
    let doc = test::mk_doc(~"impl int { fn f() { } }");
    assert doc.cratemod().impls()[0].methods[0].name == ~"f";
}

fn TyDoc_from_ty(
    ItemDoc: doc::ItemDoc
) -> doc::TyDoc {
    {
        item: ItemDoc,
        sig: None
    }
}

#[test]
fn should_extract_tys() {
    let doc = test::mk_doc(~"type a = int;");
    assert doc.cratemod().types()[0].name() == ~"a";
}

#[cfg(test)]
mod test {

    fn mk_doc(source: ~str) -> doc::Doc {
        let ast = parse::from_str(source);
        extract(ast, ~"")
    }

    #[test]
    fn extract_empty_crate() {
        let doc = mk_doc(~"");
        assert vec::is_empty(doc.cratemod().mods());
        assert vec::is_empty(doc.cratemod().fns());
    }

    #[test]
    fn extract_mods() {
        let doc = mk_doc(~"mod a { mod b { } mod c { } }");
        assert doc.cratemod().mods()[0].name() == ~"a";
        assert doc.cratemod().mods()[0].mods()[0].name() == ~"b";
        assert doc.cratemod().mods()[0].mods()[1].name() == ~"c";
    }

    #[test]
    fn extract_foreign_mods() {
        let doc = mk_doc(~"extern mod a { }");
        assert doc.cratemod().nmods()[0].name() == ~"a";
    }

    #[test]
    fn extract_fns_from_foreign_mods() {
        let doc = mk_doc(~"extern mod a { fn a(); }");
        assert doc.cratemod().nmods()[0].fns[0].name() == ~"a";
    }

    #[test]
    fn extract_mods_deep() {
        let doc = mk_doc(~"mod a { mod b { mod c { } } }");
        assert doc.cratemod().mods()[0].mods()[0].mods()[0].name() == ~"c";
    }

    #[test]
    fn extract_should_set_mod_ast_id() {
        let doc = mk_doc(~"mod a { }");
        assert doc.cratemod().mods()[0].id() != 0;
    }

    #[test]
    fn extract_fns() {
        let doc = mk_doc(
            ~"fn a() { } \
             mod b { fn c() { } }");
        assert doc.cratemod().fns()[0].name() == ~"a";
        assert doc.cratemod().mods()[0].fns()[0].name() == ~"c";
    }

    #[test]
    fn extract_should_set_fn_ast_id() {
        let doc = mk_doc(~"fn a() { }");
        assert doc.cratemod().fns()[0].id() != 0;
    }

    #[test]
    fn extract_should_use_default_crate_name() {
        let source = ~"";
        let ast = parse::from_str(source);
        let doc = extract(ast, ~"burp");
        assert doc.cratemod().name() == ~"burp";
    }

    #[test]
    fn extract_from_seq_srv() {
        let source = ~"";
        do astsrv::from_str(source) |srv| {
            let doc = from_srv(srv, ~"name");
            assert doc.cratemod().name() == ~"name";
        }
    }
}
