use std::collections::HashMap;

use rustc_hir::def::{DefKind, Res};
use rustc_hir::intravisit::{walk_expr, walk_item, Visitor};
use rustc_hir::{Expr, ExprKind, ItemKind};
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_middle::query::queries::def_kind;
use rustc_middle::query::Key;
use rustc_middle::ty::{TyCtxt, TyKind};
use rustc_abi::ExternAbi;

use log::{debug, warn};

use crate::data::{FnData, InvocData, ParamData};
use crate::utils::path_expr_segments;

pub struct Analyzer<'tcx> {
    /// The type context.
    pub tcx: TyCtxt<'tcx>,

    /// Map from C functions to more information about them.
    pub fns: HashMap<String, FnData>,

    /// List of invocations we find.
    pub invocs: Vec<InvocData>
}

impl<'tcx> Analyzer<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            fns: HashMap::new(),
            invocs: Vec::new()
        }
    }

    fn visit_expr_call(&mut self, fun: &Expr, _args: &[Expr]) -> () {
        let owner_id = fun.hir_id.owner;
        let owner_def_kind = self.tcx.def_kind(owner_id);

        if !owner_def_kind.is_fn_like() {
            return
        }

        let local_def_id = owner_id.to_def_id().as_local().unwrap();

        if let ExprKind::Path(qpath) = fun.kind {
            let typeck_results = self.tcx.typeck(local_def_id);
        
            if let Res::Def(def_kind, def_id) = typeck_results.qpath_res(&qpath, fun.hir_id) {
                if def_kind == DefKind::Fn {
                    let fn_name = self.tcx.def_path_str(def_id);
                    let fn_span = self.tcx.def_span(def_id);

                    match typeck_results.expr_ty_opt(fun) {
                        Some(ty) => match ty.kind() {
                            TyKind::FnDef(..) => {
                                let binder = ty.fn_sig(self.tcx);
                                let fn_sig = binder.skip_binder();
    
                                if let ExternAbi::C { unwind: _ } = fn_sig.abi {
                                    let parameters = fn_sig
                                        .inputs()
                                        .iter()
                                        .map(|ty| {
                                            ParamData::new(ty)
                                        })
                                        .collect();
    
                                    self.fns
                                        .entry(fn_name.clone())
                                        .or_insert_with(|| FnData::new(parameters, fn_span));
    
                                    self.invocs.push(InvocData::new(fn_name, fun.span));
                                }
                            }

                            _ => warn!("Function wasn't an `FnDef`")
                        }

                        _ => warn!("Function cannot be typed")
                    }
                }
            }
        }

        // let name = path_expr_segments(fun);

        // if name.is_empty() || name[0] == "libc" {
        //     return;
        // }

        // let owner_id = fun.hir_id.owner;
        // let owner_def_kind = self.tcx.def_kind(owner_id);

        // match owner_def_kind {
        //     DefKind::Fn | DefKind::AssocFn | DefKind::Closure => {
        //         let typeck_results = self.tcx.typeck(owner_id);

        //         match typeck_results.expr_ty_opt(fun) {
        //             Some(ty) => match ty.kind() {
        //                 TyKind::FnDef(..) | TyKind::FnPtr(..) => {
        //                     let binder = ty.fn_sig(self.tcx);
        //                     let fn_sig = binder.skip_binder();

        //                     if let Abi::C { unwind: _ } = fn_sig.abi {
        //                         let parameters = fn_sig
        //                             .inputs()
        //                             .iter()
        //                             .map(|ty| {
        //                                 ParamData::new(ty)
        //                                     .with_property("is_mutable_ptr", ty.is_mutable_ptr())
        //                                     .with_property("is_unsafe_ptr", ty.is_unsafe_ptr())
        //                                     .with_property("is_any_ptr", ty.is_any_ptr())
        //                             })
        //                             .collect();

        //                         self.fns
        //                             .entry(name.join("::"))
        //                             .or_insert_with(|| FnData::new(parameters));

        //                         self.invocs.push(InvocData::new(name.join("::"), fun.span));
        //                     }
        //                 }

        //                 _ => warn!("Function wasn't an `FnDef` or an `FnPtr`"),
        //             },

        //             _ => warn!("Function cannot be typed."),
        //         }
        //     }
        //     _ => {}
        // }
    }
}

impl<'tcx> Visitor<'tcx> for Analyzer<'tcx> {
    type NestedFilter = OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.tcx
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> Self::Result {
        match expr.kind {
            ExprKind::Call(fun, args) => self.visit_expr_call(fun, args),
            _ => walk_expr(self, expr),
        }
    }
}
