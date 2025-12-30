cmake_minimum_required(VERSION 3.13.4)
project(LLVMCallFinder)

# Set the C++ standard to 17, as required by modern LLVM versions.
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find the LLVM package.
# This requires that you have LLVM installed and that its CMake
# modules are findable (e.g., by setting -DLLVM_DIR in your cmake command).
find_package(LLVM REQUIRED CONFIG)

# Find required packages for Neo4j client
find_package(PkgConfig REQUIRED)
find_package(CURL REQUIRED)

# Find jsoncpp using pkg-config
pkg_check_modules(JSONCPP REQUIRED jsoncpp)

# Add LLVM's include directories to our project.
include_directories(${LLVM_INCLUDE_DIRS})
add_definitions(${LLVM_DEFINITIONS})

# Add the executable target with Neo4j files included.
add_executable(call-finder 
    main.cpp
    neo4j.cpp
)

# Link the executable against the necessary LLVM libraries.
# NOTE: The "IR" component was renamed to "Core" in newer LLVM versions.
llvm_map_components_to_libnames(llvm_libs Core IRReader Support)

# Link both LLVM and Neo4j dependencies
target_link_libraries(call-finder 
    ${llvm_libs}
    ${CURL_LIBRARIES} 
    ${JSONCPP_LIBRARIES}
)

# Include directories for Neo4j dependencies
target_include_directories(call-finder PRIVATE 
    ${CURL_INCLUDE_DIRS}
    ${JSONCPP_INCLUDE_DIRS}
)

# Compiler-specific options for jsoncpp
target_compile_options(call-finder PRIVATE 
    ${JSONCPP_CFLAGS_OTHER}
)

# Optional: Print configuration information for debugging
message(STATUS "LLVM version: ${LLVM_PACKAGE_VERSION}")
message(STATUS "LLVM include dirs: ${LLVM_INCLUDE_DIRS}")
message(STATUS "CURL libraries: ${CURL_LIBRARIES}")
message(STATUS "JSONCPP libraries: ${JSONCPP_LIBRARIES}")

# Optional: Add debug configuration
option(DEBUG_BUILD "Build with debug information" OFF)
if(DEBUG_BUILD)
    target_compile_options(call-finder PRIVATE -g -DDEBUG)
endif()