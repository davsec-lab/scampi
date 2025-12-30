use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass, LintContext, LintPass};
use rustc_middle::ty;
use rustc_session::declare_lint;

declare_lint! {
    pub RAW_PTR_IN_CONSTRUCTOR,
    Deny,
    "Checks for constructors that take raw pointers as arguments"
}

// Define a struct for your lint pass.
// It can just be a unit struct if it doesn't need to hold state.
#[derive(Copy, Clone)]
pub struct RawPtrConstructor;

// Implement LintPass for your struct, associating it with the lint you declared.
impl LintPass for RawPtrConstructor {
    fn get_lints(&self) -> rustc_lint::LintVec {
        vec![RAW_PTR_IN_CONSTRUCTOR]
    }

    fn name(&self) -> &'static str {
        "RawPtrConstructor"
    }
}

impl<'tcx> LateLintPass<'tcx> for RawPtrConstructor {
    /// This method is called for each item inside an `impl` block (e.g., methods).
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx hir::ImplItem<'tcx>) {
        // 1. We only care about methods/functions.
        if let hir::ImplItemKind::Fn(sig, _) = impl_item.kind {
            // 2. Get the DefId of the parent `impl` block to find out what `Self` is.
            let impl_def_id = cx.tcx.parent(impl_item.owner_id.to_def_id());
            let self_ty = cx.tcx.type_of(impl_def_id).skip_binder();

            // 3. Get the type-checked signature of the function itself.
            let fn_def_id = impl_item.owner_id.to_def_id();
            let fn_sig = cx.tcx.fn_sig(fn_def_id).skip_binder();

            // 4. Compare the function's return type with the `Self` type.
            let return_ty = fn_sig.output().skip_binder();
            if return_ty != self_ty {
                return; // It doesn't return `Self`, so we don't consider it a constructor.
            }

            // 5. It IS a constructor! Now we perform the original check for raw pointers.
            let param_tys = fn_sig.inputs();
            let hir_params = sig.decl.inputs;

            for (param_ty, hir_ty) in param_tys.iter().zip(hir_params.iter()) {
                if let ty::TyKind::RawPtr(..) = param_ty.skip_binder().kind() {
                    cx.span_lint(
                        RAW_PTR_IN_CONSTRUCTOR,
                        hir_ty.span,
                        |diag| {
                            // The `diag` object is a builder.
                            // The main message is taken from your `declare_lint!` macro.
                            // You use the builder to add extra details, like help text.
                            diag.help("Consider using a reference, smart pointer, or another owned type instead.");
                        },
                    );
                }
            }
        }
    }
}