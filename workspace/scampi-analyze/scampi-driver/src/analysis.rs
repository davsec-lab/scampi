use std::collections::HashMap;

use rustc_abi::ExternAbi;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::intravisit::{walk_expr, Visitor};
use rustc_hir::{Expr, ExprKind};
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_middle::ty::{TyCtxt, TyKind};

use log::warn;

use crate::data::{FnData, InvocData, ParamData};

pub struct Analyzer<'tcx> {
    /// The type context.
    pub tcx: TyCtxt<'tcx>,

    /// Map from C functions to more information about them.
    pub fns: HashMap<String, FnData>,

    /// List of invocations we find.
    pub invocs: Vec<InvocData>,

    // The name of the current workspace.
    pub workspace: Option<String>,

    // The name of the current crate.
    pub crate_name: String,
}

impl<'tcx> Analyzer<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, workspace: Option<String>, crate_name: String) -> Self {
        Self {
            tcx,
            fns: HashMap::new(),
            invocs: Vec::new(),
            workspace,
            crate_name,
        }
    }

    fn visit_expr_call(&mut self, fun: &Expr, _args: &[Expr]) -> () {
        let owner_id = fun.hir_id.owner;
        let owner_def_kind = self.tcx.def_kind(owner_id);

        if !owner_def_kind.is_fn_like() {
            return;
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
                                        .map(|ty| ParamData::new(ty))
                                        .collect();

                                    let source_crate =
                                        self.tcx.crate_name(def_id.krate).to_string();

                                    self.fns.entry(fn_name.clone()).or_insert_with(|| {
                                        FnData::new(parameters, fn_span, source_crate)
                                    });

                                    self.invocs.push(InvocData::new(
                                        fn_name,
                                        fun.span,
                                        self.workspace.clone(),
                                        self.crate_name.clone(),
                                    ));
                                }
                            }

                            _ => warn!("Function wasn't an `FnDef`"),
                        },

                        _ => warn!("Function cannot be typed"),
                    }
                }
            }
        }
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
