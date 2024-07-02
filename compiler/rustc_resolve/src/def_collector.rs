use std::mem;

use rustc_ast::mut_visit::FnKind;
use rustc_ast::ptr::P;
use rustc_ast::visit::AssocCtxt;
use rustc_ast::*;
use rustc_ast_pretty::pprust;
use rustc_data_structures::flat_map_in_place::FlatMapInPlace;
use rustc_expand::expand::AstFragment;
use rustc_hir as hir;
use rustc_hir::def::{CtorKind, CtorOf, DefKind};
use rustc_hir::def_id::LocalDefId;
use rustc_span::Span;
use rustc_span::hygiene::LocalExpnId;
use rustc_span::symbol::{Symbol, kw, sym};
use smallvec::{SmallVec, smallvec};
use tracing::debug;

use crate::{ImplTraitContext, InvocationParent, PendingAnonConstInfo, Resolver};

pub(crate) fn collect_definitions(
    resolver: &mut Resolver<'_, '_>,
    fragment: &mut AstFragment,
    expansion: LocalExpnId,
) {
    let InvocationParent { parent_def, pending_anon_const_info, impl_trait_context, in_attr } =
        resolver.invocation_parents[&expansion];
    let mut visitor = DefCollector {
        resolver,
        parent_def,
        pending_anon_const_info,
        expansion,
        impl_trait_context,
        in_attr,
    };
    fragment.mut_visit_with(&mut visitor);
}

/// Creates `DefId`s for nodes in the AST.
struct DefCollector<'a, 'ra, 'tcx> {
    resolver: &'a mut Resolver<'ra, 'tcx>,
    parent_def: LocalDefId,
    /// If we have an anon const that consists of a macro invocation, e.g. `Foo<{ m!() }>`,
    /// we need to wait until we know what the macro expands to before we create the def for
    /// the anon const. That's because we lower some anon consts into `hir::ConstArgKind::Path`,
    /// which don't have defs.
    ///
    /// See `Self::visit_anon_const()`.
    pending_anon_const_info: Option<PendingAnonConstInfo>,
    impl_trait_context: ImplTraitContext,
    in_attr: bool,
    expansion: LocalExpnId,
}

impl<'a, 'ra, 'tcx> DefCollector<'a, 'ra, 'tcx> {
    fn create_def(
        &mut self,
        node_id: NodeId,
        name: Symbol,
        def_kind: DefKind,
        span: Span,
    ) -> LocalDefId {
        let parent_def = self.parent_def;
        debug!(
            "create_def(node_id={:?}, def_kind={:?}, parent_def={:?})",
            node_id, def_kind, parent_def
        );
        self.resolver
            .create_def(
                parent_def,
                node_id,
                name,
                def_kind,
                self.expansion.to_expn_id(),
                span.with_parent(None),
            )
            .def_id()
    }

    fn with_parent<R, F: FnOnce(&mut Self) -> R>(&mut self, parent_def: LocalDefId, f: F) -> R {
        let orig_parent_def = mem::replace(&mut self.parent_def, parent_def);
        let res = f(self);
        self.parent_def = orig_parent_def;
        res
    }

    fn with_impl_trait<R, F: FnOnce(&mut Self) -> R>(
        &mut self,
        impl_trait_context: ImplTraitContext,
        f: F,
    ) -> R {
        let orig_itc = mem::replace(&mut self.impl_trait_context, impl_trait_context);
        let res = f(self);
        self.impl_trait_context = orig_itc;
        res
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn collect_field(&mut self, field: FieldDef, index: Option<usize>) -> SmallVec<[FieldDef; 1]> {
        let index = |this: &Self| {
            index.unwrap_or_else(|| {
                let node_id = NodeId::placeholder_from_expn_id(this.expansion);
                this.resolver.placeholder_field_indices[&node_id]
            })
        };

        if field.is_placeholder {
            let old_index = self.resolver.placeholder_field_indices.insert(field.id, index(self));
            assert!(old_index.is_none(), "placeholder field index is reset for a node ID");
            self.visit_macro_invoc(field.id);
            return smallvec![field];
        } else {
            let name = field.ident.map_or_else(|| sym::integer(index(self)), |ident| ident.name);
            let def = self.create_def(field.id, name, DefKind::Field, field.span);
            self.with_parent(def, |this| mut_visit::walk_flat_map_field_def(this, field))
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn visit_macro_invoc(&mut self, id: NodeId) {
        let id = id.placeholder_to_expn_id();
        let pending_anon_const_info = self.pending_anon_const_info.take();
        let old_parent = self.resolver.invocation_parents.insert(id, InvocationParent {
            parent_def: self.parent_def,
            pending_anon_const_info,
            impl_trait_context: self.impl_trait_context,
            in_attr: self.in_attr,
        });
        assert!(old_parent.is_none(), "parent `LocalDefId` is reset for an invocation");
    }

    /// Determines whether the const argument `AnonConst` is a simple macro call, optionally
    /// surrounded with braces.
    ///
    /// If this const argument *is* a trivial macro call then the id for the macro call is
    /// returned along with the information required to build the anon const's def if
    /// the macro call expands to a non-trivial expression.
    fn is_const_arg_trivial_macro_expansion(
        &self,
        anon_const: &'a AnonConst,
    ) -> Option<PendingAnonConstInfo> {
        let (block_was_stripped, expr) = anon_const.value.maybe_unwrap_block();
        match expr {
            Expr { kind: ExprKind::MacCall(..), .. } => Some(PendingAnonConstInfo {
                id: anon_const.id,
                span: anon_const.value.span,
                block_was_stripped,
            }),
            _ => None,
        }
    }

    /// Determines whether the expression `const_arg_sub_expr` is a simple macro call, sometimes
    /// surrounded with braces if a set of braces has not already been entered. This is required
    /// as `{ N }` is treated as equivalent to a bare parameter `N` whereas `{{ N }}` is treated as
    /// a real block expression and is lowered to an anonymous constant which is not allowed to use
    /// generic parameters.
    ///
    /// If this expression is a trivial macro call then the id for the macro call is
    /// returned along with the information required to build the anon const's def if
    /// the macro call expands to a non-trivial expression.
    fn is_const_arg_sub_expr_trivial_macro_expansion(
        &self,
        const_arg_sub_expr: &'a Expr,
    ) -> Option<(PendingAnonConstInfo, NodeId)> {
        let pending_anon = self.pending_anon_const_info.unwrap_or_else(||
            panic!("Checking expr is trivial macro call without having entered anon const: `{const_arg_sub_expr:?}`"),
        );

        let (block_was_stripped, expr) = if pending_anon.block_was_stripped {
            (true, const_arg_sub_expr)
        } else {
            const_arg_sub_expr.maybe_unwrap_block()
        };

        match expr {
            Expr { kind: ExprKind::MacCall(..), id, .. } => {
                Some((PendingAnonConstInfo { block_was_stripped, ..pending_anon }, *id))
            }
            _ => None,
        }
    }
}

impl<'a, 'ra, 'tcx> mut_visit::MutVisitor for DefCollector<'a, 'ra, 'tcx> {
    fn visit_span(&mut self, span: &mut Span) {
        if self.resolver.tcx.sess.opts.incremental.is_some() {
            *span = span.with_parent(Some(self.parent_def));
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn flat_map_item(&mut self, mut i: P<Item>) -> SmallVec<[P<Item>; 1]> {
        // Pick the def data. This need not be unique, but the more
        // information we encapsulate into, the better
        let mut opt_macro_data = None;
        let def_kind = match &i.kind {
            ItemKind::Impl(i) => DefKind::Impl { of_trait: i.of_trait.is_some() },
            ItemKind::ForeignMod(..) => DefKind::ForeignMod,
            ItemKind::Mod(..) => DefKind::Mod,
            ItemKind::Trait(..) => DefKind::Trait,
            ItemKind::TraitAlias(..) => DefKind::TraitAlias,
            ItemKind::Enum(..) => DefKind::Enum,
            ItemKind::Struct(..) => DefKind::Struct,
            ItemKind::Union(..) => DefKind::Union,
            ItemKind::ExternCrate(..) => DefKind::ExternCrate,
            ItemKind::TyAlias(..) => DefKind::TyAlias,
            ItemKind::Static(s) => DefKind::Static {
                safety: hir::Safety::Safe,
                mutability: s.mutability,
                nested: false,
            },
            ItemKind::Const(..) => DefKind::Const,
            ItemKind::Fn(..) | ItemKind::Delegation(..) => DefKind::Fn,
            ItemKind::MacroDef(..) => {
                let macro_data = self.resolver.compile_macro(&i, self.resolver.tcx.sess.edition());
                let macro_kind = macro_data.ext.macro_kind();
                opt_macro_data = Some(macro_data);
                DefKind::Macro(macro_kind)
            }
            ItemKind::GlobalAsm(..) => DefKind::GlobalAsm,
            ItemKind::Use(..) => DefKind::Use,
            ItemKind::MacCall(..) | ItemKind::DelegationMac(..) => {
                self.visit_macro_invoc(i.id);
                return smallvec![i];
            }
        };
        let def_id = self.create_def(i.id, i.ident.name, def_kind, i.span);

        if let Some(macro_data) = opt_macro_data {
            self.resolver.macro_map.insert(def_id.to_def_id(), macro_data);
        }

        self.with_parent(def_id, |this| {
            this.with_impl_trait(ImplTraitContext::Existential, |this| {
                let item = &mut *i;
                match &mut item.kind {
                    ItemKind::Struct(ref struct_def, _) | ItemKind::Union(ref struct_def, _) => {
                        // If this is a unit or tuple-like struct, register the constructor.
                        if let Some((ctor_kind, ctor_node_id)) = CtorKind::from_ast(struct_def) {
                            this.create_def(
                                ctor_node_id,
                                kw::Empty,
                                DefKind::Ctor(CtorOf::Struct, ctor_kind),
                                item.span,
                            );
                        }
                    }
                    _ => {}
                }
                mut_visit::walk_flat_map_item(this, i)
            })
        })
    }

    fn visit_fn(&mut self, mut fn_kind: FnKind<'_>, fn_span: Span, _: NodeId) {
        match &mut fn_kind {
            FnKind::Fn(FnSig { header, decl, span }, ref mut generics, ref mut body) => {
                // Identifier and visibility are visited as a part of the item.
                self.visit_fn_header(header);
                self.visit_generics(generics);

                // For async functions, we need to create their inner defs inside of a
                // closure to match their desugared representation. Besides that,
                // we must mirror everything that `visit::walk_fn` below does.
                let FnDecl { inputs, output } = &mut **decl;
                inputs.flat_map_in_place(|param| self.flat_map_param(param));

                let return_def = if let Some(coroutine_kind) = header.coroutine_kind {
                    let (return_id, return_span) = coroutine_kind.return_id();
                    self.create_def(return_id, kw::Empty, DefKind::OpaqueTy, return_span)
                } else {
                    self.parent_def
                };
                self.with_parent(return_def, |this| mut_visit::walk_fn_ret_ty(this, output));

                // If this async fn has no body (i.e. it's an async fn signature in a trait)
                // then the closure_def will never be used, and we should avoid generating a
                // def-id for it.
                if let Some(body) = body {
                    let closure_def = if let Some(coroutine_kind) = header.coroutine_kind {
                        self.create_def(
                            coroutine_kind.closure_id(),
                            kw::Empty,
                            DefKind::Closure,
                            fn_span,
                        )
                    } else {
                        self.parent_def
                    };
                    self.with_parent(closure_def, |this| this.visit_block(body));
                }
                self.visit_span(span);
            }
            FnKind::Closure(binder, coroutine_kind, decl, body) => {
                self.visit_closure_binder(binder);
                self.visit_fn_decl(decl);

                // Async closures desugar to closures inside of closures, so
                // we must create two defs.
                let closure_def = if let Some(coroutine_kind) = coroutine_kind {
                    self.create_def(
                        coroutine_kind.closure_id(),
                        kw::Empty,
                        DefKind::Closure,
                        fn_span,
                    )
                } else {
                    self.parent_def
                };
                self.with_parent(closure_def, |this| this.visit_expr(body));
            }
        }
    }

    fn visit_use_tree(&mut self, use_tree: &mut UseTree) {
        let UseTree { prefix, kind, span } = use_tree;
        self.visit_path(prefix);
        match kind {
            UseTreeKind::Simple(None) => {}
            UseTreeKind::Simple(Some(rename)) => self.visit_ident(rename),
            UseTreeKind::Nested { items, span } => {
                for (tree, id) in items {
                    self.visit_id(id);
                    // HIR lowers use trees as a flat stream of `ItemKind::Use`.
                    // This means all the def-ids must be parented to the module,
                    // and not to `self.parent_def` which is the topmost `use` item.
                    self.resolver.create_def(
                        self.resolver.tcx.local_parent(self.parent_def),
                        *id,
                        kw::Empty,
                        DefKind::Use,
                        self.expansion.to_expn_id(),
                        span.with_parent(None),
                    );
                    self.visit_use_tree(tree);
                }
                self.visit_span(span);
            }
            UseTreeKind::Glob => {}
        }
        self.visit_span(span);
    }

    fn flat_map_foreign_item(&mut self, fi: P<ForeignItem>) -> SmallVec<[P<ForeignItem>; 1]> {
        let def_kind = match fi.kind {
            ForeignItemKind::Static(box StaticItem { ty: _, mutability, expr: _, safety }) => {
                let safety = match safety {
                    ast::Safety::Unsafe(_) | ast::Safety::Default => hir::Safety::Unsafe,
                    ast::Safety::Safe(_) => hir::Safety::Safe,
                };

                DefKind::Static { safety, mutability, nested: false }
            }
            ForeignItemKind::Fn(_) => DefKind::Fn,
            ForeignItemKind::TyAlias(_) => DefKind::ForeignTy,
            ForeignItemKind::MacCall(_) => {
                self.visit_macro_invoc(fi.id);
                return smallvec![fi];
            }
        };

        let def = self.create_def(fi.id, fi.ident.name, def_kind, fi.span);

        self.with_parent(def, |this| mut_visit::walk_flat_map_item(this, fi))
    }

    fn flat_map_variant(&mut self, v: Variant) -> SmallVec<[Variant; 1]> {
        if v.is_placeholder {
            self.visit_macro_invoc(v.id);
            return smallvec![v];
        }
        let def = self.create_def(v.id, v.ident.name, DefKind::Variant, v.span);
        self.with_parent(def, |this| {
            if let Some((ctor_kind, ctor_node_id)) = CtorKind::from_ast(&v.data) {
                this.create_def(
                    ctor_node_id,
                    kw::Empty,
                    DefKind::Ctor(CtorOf::Variant, ctor_kind),
                    v.span,
                );
            }
            mut_visit::walk_flat_map_variant(this, v)
        })
    }

    fn visit_variant_data(&mut self, data: &mut VariantData) {
        // The assumption here is that non-`cfg` macro expansion cannot change field indices.
        // It currently holds because only inert attributes are accepted on fields,
        // and every such attribute expands into a single field after it's resolved.
        let fields = match data {
            VariantData::Struct { fields, recovered: _ } => fields,
            VariantData::Tuple(fields, id) => {
                self.visit_id(id);
                fields
            }
            VariantData::Unit(id) => {
                self.visit_id(id);
                return;
            }
        };
        let mut index = 0;
        fields.flat_map_in_place(|field| {
            let field = self.collect_field(field, Some(index));
            index = index + 1;
            field
        })
    }

    fn flat_map_generic_param(&mut self, param: GenericParam) -> SmallVec<[GenericParam; 1]> {
        if param.is_placeholder {
            self.visit_macro_invoc(param.id);
            return smallvec![param];
        }
        let def_kind = match param.kind {
            GenericParamKind::Lifetime { .. } => DefKind::LifetimeParam,
            GenericParamKind::Type { .. } => DefKind::TyParam,
            GenericParamKind::Const { .. } => DefKind::ConstParam,
        };
        self.create_def(param.id, param.ident.name, def_kind, param.ident.span);

        // impl-Trait can happen inside generic parameters, like
        // ```
        // fn foo<U: Iterator<Item = impl Clone>>() {}
        // ```
        //
        // In that case, the impl-trait is lowered as an additional generic parameter.
        self.with_impl_trait(ImplTraitContext::Universal, |this| {
            mut_visit::walk_flat_map_generic_param(this, param)
        })
    }

    fn flat_map_assoc_item(
        &mut self,
        i: P<AssocItem>,
        _: AssocCtxt,
    ) -> SmallVec<[P<AssocItem>; 1]> {
        let def_kind = match &i.kind {
            AssocItemKind::Fn(..) | AssocItemKind::Delegation(..) => DefKind::AssocFn,
            AssocItemKind::Const(..) => DefKind::AssocConst,
            AssocItemKind::Type(..) => DefKind::AssocTy,
            AssocItemKind::MacCall(..) | AssocItemKind::DelegationMac(..) => {
                self.visit_macro_invoc(i.id);
                return smallvec![i];
            }
        };

        let span = i.span;
        let def = self.create_def(i.id, i.ident.name, def_kind, span);
        self.with_parent(def, |this| mut_visit::walk_flat_map_item(this, i))
    }

    fn visit_pat(&mut self, pat: &mut P<Pat>) {
        if let PatKind::MacCall(..) = pat.kind {
            return self.visit_macro_invoc(pat.id);
        }
        mut_visit::walk_pat(self, pat)
    }

    fn visit_anon_const(&mut self, constant: &mut AnonConst) {
        // HACK(min_generic_const_args): don't create defs for anon consts if we think they will
        // later be turned into ConstArgKind::Path's. because this is before resolve is done, we
        // may accidentally identify a construction of a unit struct as a param and not create a
        // def. we'll then create a def later in ast lowering in this case. the parent of nested
        // items will be messed up, but that's ok because there can't be any if we're just looking
        // for bare idents.

        if let Some(pending_anon) = self.is_const_arg_trivial_macro_expansion(constant) {
            self.pending_anon_const_info = Some(pending_anon);
            return mut_visit::walk_anon_const(self, constant);
        } else if constant.value.is_potential_trivial_const_arg(true) {
            return mut_visit::walk_anon_const(self, constant);
        }

        let def = self.create_def(constant.id, kw::Empty, DefKind::AnonConst, constant.value.span);
        self.with_parent(def, |this| mut_visit::walk_anon_const(this, constant));
    }

    fn visit_expr(&mut self, expr: &mut P<Expr>) {
        // If we're visiting the expression of a const argument that was a macro call then
        // check if it is *still* unknown whether it is a trivial const arg or not. If so
        // recurse into the macro call and delay creating the anon const def until expansion.
        if self.pending_anon_const_info.is_some()
            && let Some((pending_anon, macro_invoc)) =
                self.is_const_arg_sub_expr_trivial_macro_expansion(expr)
        {
            self.pending_anon_const_info = Some(pending_anon);
            return self.visit_macro_invoc(macro_invoc);
        }

        // See self.pending_anon_const_info for explanation
        let parent_def = self
            .pending_anon_const_info
            .take()
            // If we already stripped away a set of braces then do not do it again when determining
            // if the macro expanded to a trivial const arg. This arises in cases such as:
            // `Foo<{ bar!() }>` where `bar!()` expands to `{ N }`. This should not be considered a
            // trivial const argument even though `{ N }` by itself *is*.
            .filter(|pending_anon| {
                !expr.is_potential_trivial_const_arg(!pending_anon.block_was_stripped)
            })
            .map(|pending_anon| {
                self.create_def(pending_anon.id, kw::Empty, DefKind::AnonConst, pending_anon.span)
            })
            .unwrap_or(self.parent_def);

        let expr = &mut **expr;
        self.with_parent(parent_def, |this| {
            let parent_def = match expr.kind {
                ExprKind::MacCall(..) => return this.visit_macro_invoc(expr.id),
                ExprKind::Closure(..) | ExprKind::Gen(..) => {
                    this.create_def(expr.id, kw::Empty, DefKind::Closure, expr.span)
                }
                ExprKind::ConstBlock(ref mut constant) => {
                    mut_visit::visit_attrs(this, &mut expr.attrs);
                    let def = this.create_def(
                        constant.id,
                        kw::Empty,
                        DefKind::InlineConst,
                        constant.value.span,
                    );
                    this.with_parent(def, |this| mut_visit::walk_anon_const(this, constant));
                    this.visit_span(&mut expr.span);
                    return;
                }
                _ => this.parent_def,
            };

            this.with_parent(parent_def, |this| mut_visit::walk_expr(this, expr))
        })
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn visit_ty(&mut self, ty: &mut P<Ty>) {
        match &ty.kind {
            TyKind::MacCall(..) => self.visit_macro_invoc(ty.id),
            TyKind::ImplTrait(id, _) => {
                // HACK: pprust breaks strings with newlines when the type
                // gets too long. We don't want these to show up in compiler
                // output or built artifacts, so replace them here...
                // Perhaps we should instead format APITs more robustly.
                let name = Symbol::intern(&pprust::ty_to_string(ty).replace('\n', " "));
                let kind = match self.impl_trait_context {
                    ImplTraitContext::Universal => DefKind::TyParam,
                    ImplTraitContext::Existential => DefKind::OpaqueTy,
                };
                let id = self.create_def(*id, name, kind, ty.span);
                match self.impl_trait_context {
                    // Do not nest APIT, as we desugar them as `impl_trait: bounds`,
                    // so the `impl_trait` node is not a parent to `bounds`.
                    ImplTraitContext::Universal => mut_visit::walk_ty(self, ty),
                    ImplTraitContext::Existential => {
                        self.with_parent(id, |this| mut_visit::walk_ty(this, ty))
                    }
                };
            }
            _ => mut_visit::walk_ty(self, ty),
        }
    }

    fn flat_map_stmt(&mut self, stmt: Stmt) -> SmallVec<[Stmt; 1]> {
        if let StmtKind::MacCall(..) = stmt.kind {
            self.visit_macro_invoc(stmt.id);
            return smallvec![stmt];
        }
        mut_visit::walk_flat_map_stmt(self, stmt)
    }

    fn flat_map_arm(&mut self, arm: Arm) -> SmallVec<[Arm; 1]> {
        if arm.is_placeholder {
            self.visit_macro_invoc(arm.id);
            return smallvec![arm];
        }
        mut_visit::walk_flat_map_arm(self, arm)
    }

    fn flat_map_expr_field(&mut self, f: ExprField) -> SmallVec<[ExprField; 1]> {
        if f.is_placeholder {
            self.visit_macro_invoc(f.id);
            return smallvec![f];
        }
        mut_visit::walk_flat_map_expr_field(self, f)
    }

    fn flat_map_pat_field(&mut self, fp: PatField) -> SmallVec<[PatField; 1]> {
        if fp.is_placeholder {
            self.visit_macro_invoc(fp.id);
            return smallvec![fp];
        }
        mut_visit::walk_flat_map_pat_field(self, fp)
    }

    fn flat_map_param(&mut self, p: Param) -> SmallVec<[Param; 1]> {
        if p.is_placeholder {
            self.visit_macro_invoc(p.id);
            return smallvec![p];
        }
        self.with_impl_trait(ImplTraitContext::Universal, |this| {
            mut_visit::walk_flat_map_param(this, p)
        })
    }

    // This method is called only when we are visiting an individual field
    // after expanding an attribute on it.
    fn flat_map_field_def(&mut self, field: FieldDef) -> SmallVec<[FieldDef; 1]> {
        self.collect_field(field, None)
    }

    fn visit_crate(&mut self, krate: &mut Crate) {
        if krate.is_placeholder {
            return self.visit_macro_invoc(krate.id);
        }
        mut_visit::walk_crate(self, krate)
    }

    fn visit_attribute(&mut self, attr: &mut Attribute) {
        let orig_in_attr = mem::replace(&mut self.in_attr, true);
        mut_visit::walk_attribute(self, attr);
        self.in_attr = orig_in_attr;
    }
}
