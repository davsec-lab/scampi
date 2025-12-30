#include "llvm/IR/BasicBlock.h"
#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Instruction.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/IRReader/IRReader.h"
#include "llvm/Support/SourceMgr.h"
#include "llvm/Support/raw_ostream.h"
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>
#include "neo4j.h"

using namespace std;

std::vector<std::string> collectArgs(llvm::Function *fn) {
  std::vector<std::string> args;
  args.reserve(fn->arg_size());

  std::string typeStr;
  for (const auto &arg : fn->args()) {
    typeStr.clear();
    llvm::raw_string_ostream rso(typeStr);
    arg.getType()->print(rso);
    rso.flush();
    args.push_back(typeStr);
  }

  return args;
}

void printArgs(vector<string> args) {
  for (size_t i = 0; i < args.size(); ++i) {
    llvm::outs() << "- Arg " << i << ": " << args[i] << "\n";
  }
}

string get_def_loc(llvm::Function *fn) {
    std::string loc = "(location unknown)";
    if (llvm::DISubprogram *subprogram = fn->getSubprogram()) {
      loc = subprogram->getDirectory().str() + "/" +
                       subprogram->getFilename().str() + ":" +
                       std::to_string(subprogram->getLine());
    }

    return loc;
}

void register_fn(Neo4jClient * client, string crate, llvm::Function *fn, bool useCratePrefix = false) {
  string abi = "C";

  string query;
  llvm::raw_string_ostream rso(query);
  
  // Determine source information for the function definition
  unsigned defLine = 0;
  unsigned defColumn = 0; // Column may be unavailable; default to 0
  string defFile = "(file unknown)";
  if (llvm::DISubprogram *subprogram = fn->getSubprogram()) {
    defLine = subprogram->getLine();
    // llvm::DISubprogram does not provide column information; keep 0
    std::string dir = subprogram->getDirectory().str();
    std::string fname = subprogram->getFilename().str();
    if (!dir.empty()) {
      defFile = dir + "/" + fname;
    } else {
      defFile = fname;
    }
  }
  
  // Format function name with optional crate prefix
  string functionName = fn->getName().str();
  if (useCratePrefix) {
    functionName = crate + "::" + functionName;
  }

  rso << "MERGE (f:Fn:" << abi << " {name: \"" << functionName << "\", crate: \"" << crate << "\", safe: false }) "
      << "SET f.line = " << defLine << ", f.column = " << defColumn << ", f.file = \"" << defFile << "\" "
      << "RETURN f";

  llvm::outs() << "QUERY: " << rso.str() << "\n";

  client->submitQuery(rso.str());
}

void register_edge(Neo4jClient * client, string caller, string callee, string span, string crate, bool useCratePrefix = false) {
  string query;
  llvm::raw_string_ostream rso(query);

  if (useCratePrefix) {
    caller = crate + "::" + caller;
    callee = crate + "::" + callee;
  }

  rso << "MATCH (a:Fn), (b:Fn) WHERE a.name = \"" << caller << "\" AND b.name = \"" << callee << "\""
      << "MERGE (a)-[r:CALLS { span: \"" << span << "\" }]->(b) "
      << "RETURN r";

  llvm::outs() << "QUERY: " << rso.str() << "\n";

  client->submitQuery(rso.str());
}

int main(int argc, char **argv) {
  if (argc < 3) {
    llvm::errs() << "Usage: " << argv[0] << " <path-to-llvm-ir-file.ll> <name-of-package> [--prefixed]\n";
    return 1;
  }

  char *filePath = argv[1];
  char *package = argv[2];
  bool useCratePrefix = false;
  
  // Check for optional --prefixed flag
  for (int i = 3; i < argc; i++) {
    if (std::string(argv[i]) == "--prefixed") {
      useCratePrefix = true;
      break;
    }
  }

  // Holds global state for the IR
  llvm::LLVMContext context;

  // For error and warning reporting
  llvm::SMDiagnostic err;

  // Parse the IR file into a module
  std::unique_ptr<llvm::Module> module = llvm::parseIRFile(filePath, err, context);

  // Check if parsing was successful
  if (!module) {
    err.print(argv[0], llvm::errs());
    return 1;
  }

  // Create Neo4j client instance
  Neo4jClient client("http://host.docker.internal:7474", "neo4j", "Qwerty12!");

  // Test connection
  llvm::outs() << "Testing connection...\n";
  if (client.testConnection()) {
      llvm::outs() << "✓ Connected to Neo4j successfully!\n";
  } else {
      llvm::outs() << "✗ Failed to connect to Neo4j\n";
      return 1;
  }

  // Iterate over every function defined in the module
  for (llvm::Function &callerFunction : *module) {
    if (callerFunction.isIntrinsic())
      continue;

    string callerLocation = get_def_loc(&callerFunction);

    llvm::outs() << "[CALLER] "
                 << callerFunction.getName()
                 << " ("
                 << callerLocation
                 << ")"
                 << "\n";

    vector<string> callerArgs = collectArgs(&callerFunction);
    printArgs(callerArgs);

    register_fn(&client, package, &callerFunction, useCratePrefix);

    // Iterate over every basic block within the function
    for (llvm::BasicBlock &basicBlock : callerFunction) {

      // Iterate over every instruction within the basic block
      for (llvm::Instruction &instruction : basicBlock) {

        // We are only interested in 'call' instructions
        if (auto *callInst = llvm::dyn_cast<llvm::CallInst>(&instruction)) {
          // Get the function being called (the "callee")
          llvm::Function *calleeFunction = callInst->getCalledFunction();

          // Might be an indirect call (for example, function pointer call)
          if (!calleeFunction)
            continue;

          if (calleeFunction->isIntrinsic())
            continue;

          register_fn(&client, package, calleeFunction, useCratePrefix);

          // Debug location contains source file, line, and column info
          string callLocation = "(unknown)";

          if (const llvm::DILocation *debugLoc = instruction.getDebugLoc()) {
            unsigned line = debugLoc->getLine();
            unsigned column = debugLoc->getColumn();
            llvm::StringRef file = debugLoc->getFilename();
            llvm::StringRef dir = debugLoc->getDirectory();

            // Print the formatted output
            llvm::outs() << "[CALL] "
                         << callerFunction.getName()
                         << " called "
                         << calleeFunction->getName()
                         << " on line "
                         << line
                         << ", column "
                         << column
                         << " of "
                         << dir
                         << "/"
                         << file
                         << "\n";
              
            callLocation = dir.str() + "/" + file.str() + ":" + to_string(line) + ":" + to_string(column);
          } else {
            llvm::outs() << callerFunction.getName()
                         << " called "
                         << calleeFunction->getName()
                         << " (no source location available)"
                         << "\n";
          }

          register_edge(&client, callerFunction.getFunction().getName().str(), calleeFunction->getFunction().getName().str(), callLocation, package, useCratePrefix);

          std::string calleeLocation = "(location unknown)";
          if (llvm::DISubprogram *subprogram = calleeFunction->getSubprogram()) {
            calleeLocation = subprogram->getDirectory().str() + "/" +
                             subprogram->getFilename().str() + ":" +
                             std::to_string(subprogram->getLine());
          }

          llvm::outs() << "[CALLEE] "
                       << calleeFunction->getName()
                       << " ("
                       << calleeLocation
                       << ")"
                       << "\n";

          vector<string> calleeArgs = collectArgs(calleeFunction);
          printArgs(calleeArgs);
        }
      }
    }
  }

  // Flush any remaining queries before exiting
  client.flushQueries();

  return 0;
}
