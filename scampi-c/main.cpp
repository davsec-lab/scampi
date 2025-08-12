#include "clang/AST/AST.h"
#include "clang/AST/ASTConsumer.h"
#include "clang/AST/ASTContext.h"
#include "clang/AST/RecursiveASTVisitor.h"
#include "clang/Driver/Options.h"
#include "clang/Frontend/ASTConsumers.h"
#include "clang/Frontend/FrontendActions.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/Tooling/CommonOptionsParser.h"
#include "clang/Tooling/Tooling.h"
#include "clang/Rewrite/Core/Rewriter.h"
#include "llvm/Support/raw_ostream.h"
#include "clang/AST/Expr.h"
#include "clang/AST/Type.h"
#include "clang/AST/Decl.h"
#include "clang/Basic/SourceManager.h"

using namespace clang;
using namespace clang::driver;
using namespace clang::tooling;
using namespace llvm;

// Visitor class that traverses the AST
class CAnalysisVisitor : public RecursiveASTVisitor<CAnalysisVisitor> {
private:
    ASTContext *Context;
    SourceManager *SM;

public:
    explicit CAnalysisVisitor(ASTContext *Context) 
        : Context(Context), SM(&Context->getSourceManager()) {}

    // Visit all expressions to get type information
    bool VisitExpr(Expr *E) {
        if (!SM->isInMainFile(E->getBeginLoc()))
            return true; // Skip system headers

        QualType QT = E->getType();
        std::string TypeStr = QT.getAsString();
        
        outs() << "Expression at ";
        E->getBeginLoc().print(outs(), *SM);
        outs() << ": " << getExpressionString(E) << "\n";
        outs() << "  Type: " << TypeStr << "\n";
        
        // Additional type analysis
        if (QT->isPointerType()) {
            outs() << "  -> Pointer to: " << QT->getPointeeType().getAsString() << "\n";
        }
        if (QT->isArrayType()) {
            outs() << "  -> Array element type: " << QT->getAsArrayTypeUnsafe()->getElementType().getAsString() << "\n";
        }
        if (QT->isFunctionType()) {
            outs() << "  -> Function type\n";
        }
        
        outs() << "\n";
        return true;
    }

    // Visit function declarations
    bool VisitFunctionDecl(FunctionDecl *F) {
        if (!SM->isInMainFile(F->getLocation()))
            return true;

        outs() << "Function: " << F->getNameAsString() << "\n";
        outs() << "  Return type: " << F->getReturnType().getAsString() << "\n";
        outs() << "  Parameters: ";
        
        for (unsigned i = 0; i < F->getNumParams(); ++i) {
            ParmVarDecl *Param = F->getParamDecl(i);
            outs() << Param->getType().getAsString();
            if (Param->getNameAsString().size() > 0) {
                outs() << " " << Param->getNameAsString();
            }
            if (i < F->getNumParams() - 1) outs() << ", ";
        }
        outs() << "\n";
        
        // Check if it's a definition vs declaration
        if (F->hasBody()) {
            outs() << "  Has definition\n";
        } else {
            outs() << "  Declaration only\n";
        }
        
        outs() << "\n";
        return true;
    }

    // Visit variable declarations
    bool VisitVarDecl(VarDecl *V) {
        if (!SM->isInMainFile(V->getLocation()))
            return true;

        outs() << "Variable: " << V->getNameAsString() << "\n";
        outs() << "  Type: " << V->getType().getAsString() << "\n";
        outs() << "  Storage class: ";
        
        switch (V->getStorageClass()) {
            case SC_None: outs() << "none"; break;
            case SC_Static: outs() << "static"; break;
            case SC_Extern: outs() << "extern"; break;
            case SC_Auto: outs() << "auto"; break;
            case SC_Register: outs() << "register"; break;
        }
        outs() << "\n";
        
        if (V->hasInit()) {
            outs() << "  Has initializer\n";
        }
        
        outs() << "\n";
        return true;
    }

    // Visit struct/union declarations
    bool VisitRecordDecl(RecordDecl *R) {
        if (!SM->isInMainFile(R->getLocation()))
            return true;

        outs() << (R->isStruct() ? "Struct: " : "Union: ") << R->getNameAsString() << "\n";
        
        if (R->isCompleteDefinition()) {
            outs() << "  Fields:\n";
            for (auto *Field : R->fields()) {
                outs() << "    " << Field->getType().getAsString() 
                       << " " << Field->getNameAsString() << "\n";
            }
        } else {
            outs() << "  Forward declaration\n";
        }
        
        outs() << "\n";
        return true;
    }

    // Visit typedef declarations
    bool VisitTypedefDecl(TypedefDecl *T) {
        if (!SM->isInMainFile(T->getLocation()))
            return true;

        outs() << "Typedef: " << T->getNameAsString() << " = " 
               << T->getUnderlyingType().getAsString() << "\n\n";
        return true;
    }

    // Visit call expressions to analyze function calls
    bool VisitCallExpr(CallExpr *CE) {
        if (!SM->isInMainFile(CE->getBeginLoc()))
            return true;

        FunctionDecl *Callee = CE->getDirectCallee();
        if (Callee) {
            outs() << "Function call: " << Callee->getNameAsString() << "\n";
            outs() << "  Arguments:\n";
            
            for (unsigned i = 0; i < CE->getNumArgs(); ++i) {
                Expr *Arg = CE->getArg(i);
                outs() << "    " << getExpressionString(Arg) 
                       << " (type: " << Arg->getType().getAsString() << ")\n";
            }
            outs() << "\n";
        }
        
        return true;
    }

    // Visit macro expansions (requires preprocessing info)
    bool VisitMacroExpansion(SourceLocation Loc, const MacroInfo *MI) {
        if (!SM->isInMainFile(Loc))
            return true;

        outs() << "Macro expansion at ";
        Loc.print(outs(), *SM);
        outs() << "\n";
        return true;
    }

private:
    // Helper function to get string representation of expressions
    std::string getExpressionString(Expr *E) {
        std::string Result;
        raw_string_ostream OS(Result);
        E->printPretty(OS, nullptr, Context->getPrintingPolicy());
        return OS.str();
    }
};

// Consumer that owns the visitor
class CAnalysisConsumer : public ASTConsumer {
private:
    CAnalysisVisitor Visitor;

public:
    explicit CAnalysisConsumer(ASTContext *Context) : Visitor(Context) {}

    virtual void HandleTranslationUnit(ASTContext &Context) override {
        Visitor.TraverseDecl(Context.getTranslationUnitDecl());
    }
};

// Frontend action that creates the consumer
class CAnalysisAction : public ASTFrontendAction {
public:
    virtual std::unique_ptr<ASTConsumer> CreateASTConsumer(
        CompilerInstance &Compiler, StringRef InFile) override {
        return std::make_unique<CAnalysisConsumer>(&Compiler.getASTContext());
    }
};

// Command line options
static cl::OptionCategory CAnalyzerCategory("C Analyzer options");
static cl::extrahelp CommonHelp(CommonOptionsParser::HelpMessage);

int main(int argc, const char **argv) {
    // Fix duplicate LLVM option registration error
    llvm::cl::ResetCommandLineParser();
    llvm::cl::ResetAllOptionOccurrences();
    
    auto ExpectedParser = CommonOptionsParser::create(argc, argv, CAnalyzerCategory);

    if (!ExpectedParser) {
        errs() << ExpectedParser.takeError();
        return 1;
    }
    CommonOptionsParser &OptionsParser = ExpectedParser.get();
    
    ClangTool Tool(OptionsParser.getCompilations(),
                   OptionsParser.getSourcePathList());

    // Add compiler arguments to handle macros and includes
    Tool.appendArgumentsAdjuster(getInsertArgumentAdjuster("-fsyntax-only")); // Only parse, don't generate code
    Tool.appendArgumentsAdjuster(getInsertArgumentAdjuster("-Wno-everything")); // Suppress warnings for cleaner output

    return Tool.run(newFrontendActionFactory<CAnalysisAction>().get());
}