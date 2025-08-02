use neo4rs::{query, Graph, Query};
use rustc_abi::ExternAbi;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::intravisit::{walk_expr, walk_impl_item, walk_item, Visitor};
use rustc_hir::{Expr, ExprKind, ItemKind};
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_middle::ty::{TyCtxt, TyKind};

use log::{debug, warn};
use rustc_span::Span;
use tokio::runtime::Runtime;

use crate::data::{clean_span, ParamData};

struct FunSig {
    krate: String,
    name: String,
    span: Span,
    abi: ExternAbi,
    safe: bool,
    params: Vec<ParamData>,
}

pub struct Analyzer<'tcx> {
    /// Aync runtime for executing database operations.
    rt: Runtime,

    /// Graph database connection
    graph: Graph,

    /// The type context.
    pub tcx: TyCtxt<'tcx>,

    /// The name of the current crate.
    pub crate_name: String,

    /// The current loop level (how many nested loops).
    loop_level: i32,

    /// Stack of function definitions.
    caller_name: Vec<String>,
}

impl<'tcx> Analyzer<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, crate_name: String) -> Self {
        let uri = std::env::var("SCAMPI_NEO4J_URI").unwrap();
        let password = std::env::var("SCAMPI_NEO4J_PASSWORD").unwrap();

        let rt = Runtime::new().unwrap();
        let graph = rt.block_on(Graph::new(uri, "neo4j", password)).unwrap();

        Self {
            rt: Runtime::new().unwrap(),
            graph,
            tcx,
            crate_name,
            caller_name: vec![],
            loop_level: 0,
        }
    }

    fn run_blocking_query(&mut self, query: Query) {
        self.rt
            .block_on(async {
                self.graph
                    .execute(query)
                    .await
                    .unwrap()
                    .next()
                    .await
                    .unwrap()
            })
            .unwrap();
    }

    fn register_fn(&mut self, fn_sig: FunSig) {
        let abi = match fn_sig.abi {
            ExternAbi::Rust => "Rust",
            ExternAbi::C { unwind: _ } => "C",
            _ => "Other",
        };

        let fn_query = query(&format!(
            "MERGE (f:Fn:{abi} {{name: $name, crate: $crate, safe: $safe, span: $span}}) RETURN f"
        ))
        .param("name", fn_sig.name.clone())
        .param("crate", fn_sig.krate.clone())
        .param("safe", fn_sig.safe)
        .param("span", clean_span(fn_sig.span));

        self.run_blocking_query(fn_query);

        for (i, param) in fn_sig.params.iter().enumerate() {
            let param_query =
                query("MERGE (p:Param { ty: $ty, is_mutable_ptr: $is_mutable_ptr }) RETURN p")
                    .param("ty", param.ty.clone())
                    .param("is_mutable_ptr", param.is_mutable_ptr);

            self.run_blocking_query(param_query);

            let accepts_query = query(
                "MATCH (f:Fn), (p:Param) \
                    WHERE f.name = $fn_name AND p.ty = $ty \
                    MERGE (f)-[r:ACCEPTS { i: $i }]->(p) \
                    RETURN r",
            )
            .param("fn_name", fn_sig.name.clone())
            .param("ty", param.ty.clone())
            .param("i", i as i32);

            self.run_blocking_query(accepts_query);
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

                                if self.caller_name.is_empty() {
                                    debug!("NO CALLER! At span {}", clean_span(fn_span));
                                    return;
                                }

                                let params: Vec<ParamData> = fn_sig
                                    .inputs()
                                    .iter()
                                    .map(|ty| ParamData::new(ty))
                                    .collect();

                                let fun_sig = FunSig {
                                    krate: self.tcx.crate_name(def_id.krate).to_string(),
                                    name: fn_name.clone(),
                                    span: fn_span,
                                    abi: fn_sig.abi,
                                    safe: fn_sig.safety.is_safe(),
                                    params,
                                };

                                self.register_fn(fun_sig);

                                let relationship_query = query(
                                    "MATCH (a:Fn), (b:Fn) \
                                    WHERE a.name = $caller AND b.name = $callee \
                                    MERGE (a)-[r:CALLS { span: $span, loop_level: $loop_level }]->(b) \
                                    RETURN r",
                                )
                                .param("caller", self.caller_name.last().unwrap().clone())
                                .param("callee", fn_name.clone())
                                .param("span", clean_span(fun.span))
                                .param("loop_level", self.loop_level);

                                let _ = self
                                    .rt
                                    .block_on(async { self.graph.run(relationship_query).await });
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
            ExprKind::Loop(_, _, _, _) => {
                self.loop_level += 1;

                walk_expr(self, expr);

                self.loop_level -= 1;
            }
            _ => walk_expr(self, expr),
        }
    }

    fn visit_item(&mut self, item: &'tcx rustc_hir::Item<'tcx>) -> Self::Result {
        match item.kind {
            ItemKind::Fn { sig, body, .. } => {
                let def_id = body.hir_id.owner.to_def_id();

                let fn_name = self.tcx.def_path_str(def_id);
                let fn_span = self.tcx.def_span(def_id);

                let params: Vec<ParamData> = sig
                    .decl
                    .inputs
                    .iter()
                    .map(|hir_ty| {
                        let ty = self
                            .tcx
                            .type_of(hir_ty.hir_id.owner.to_def_id())
                            .skip_binder();
                        ParamData::new(&ty)
                    })
                    .collect();

                let fun_sig = FunSig {
                    krate: self.crate_name.clone(),
                    name: fn_name.clone(),
                    span: fn_span,
                    abi: sig.header.abi,
                    safe: sig.header.is_safe(),
                    params,
                };

                self.register_fn(fun_sig);

                self.caller_name.push(fn_name);

                walk_item(self, item);

                self.caller_name.pop();
            }
            _ => {}
        };
    }

    fn visit_impl_item(&mut self, impl_item: &'tcx rustc_hir::ImplItem<'tcx>) -> Self::Result {
        match impl_item.kind {
            rustc_hir::ImplItemKind::Fn(sig, body) => {
                let def_id = body.hir_id.owner.to_def_id();

                let fn_name = self.tcx.def_path_str(def_id);
                let fn_span = self.tcx.def_span(def_id);

                let params: Vec<ParamData> = sig
                    .decl
                    .inputs
                    .iter()
                    .map(|hir_ty| {
                        let ty = self
                            .tcx
                            .type_of(hir_ty.hir_id.owner.to_def_id())
                            .skip_binder();
                        ParamData::new(&ty)
                    })
                    .collect();

                let fun_sig = FunSig {
                    krate: self.crate_name.clone(),
                    name: fn_name.clone(),
                    span: fn_span,
                    abi: sig.header.abi,
                    safe: sig.header.is_safe(),
                    params,
                };

                self.register_fn(fun_sig);

                self.caller_name.push(fn_name);

                walk_impl_item(self, impl_item);

                self.caller_name.pop();
            }
            _ => {}
        };
    }
}
