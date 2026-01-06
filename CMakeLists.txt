cmake_minimum_required(VERSION 3.14)

# Set extension name here
set(TARGET_NAME pyfunc)
set(EXTENSION_NAME ${TARGET_NAME}_extension)
set(LOADABLE_EXTENSION_NAME ${TARGET_NAME}_loadable_extension)

project(${TARGET_NAME} LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Find Python and pybind11
find_package(Python3 REQUIRED COMPONENTS Interpreter Development)
find_package(pybind11 CONFIG QUIET)

if(NOT pybind11_FOUND)
    # Fetch pybind11 if not found
    include(FetchContent)
    FetchContent_Declare(
        pybind11
        GIT_REPOSITORY https://github.com/pybind/pybind11.git
        GIT_TAG v2.11.1
    )
    FetchContent_MakeAvailable(pybind11)
endif()

# DuckDB configuration
if(NOT DEFINED DUCKDB_DIR)
    # Try to find DuckDB in common locations or fetch it
    find_path(DUCKDB_INCLUDE_DIR
        NAMES duckdb.hpp
        PATHS
            /usr/local/include
            /usr/include
            ${CMAKE_SOURCE_DIR}/duckdb/src/include
            $ENV{DUCKDB_DIR}/src/include
    )

    if(NOT DUCKDB_INCLUDE_DIR)
        message(STATUS "DuckDB not found, fetching from GitHub...")
        include(FetchContent)
        FetchContent_Declare(
            duckdb
            GIT_REPOSITORY https://github.com/duckdb/duckdb.git
            GIT_TAG v1.1.3
            GIT_SHALLOW TRUE
        )
        set(BUILD_SHELL OFF CACHE BOOL "" FORCE)
        set(BUILD_UNITTESTS OFF CACHE BOOL "" FORCE)
        set(BUILD_BENCHMARKS OFF CACHE BOOL "" FORCE)
        FetchContent_MakeAvailable(duckdb)
        set(DUCKDB_INCLUDE_DIR ${duckdb_SOURCE_DIR}/src/include)
        set(DUCKDB_LIBRARY duckdb)
    else()
        find_library(DUCKDB_LIBRARY NAMES duckdb)
    endif()
else()
    set(DUCKDB_INCLUDE_DIR ${DUCKDB_DIR}/src/include)
    find_library(DUCKDB_LIBRARY NAMES duckdb PATHS ${DUCKDB_DIR}/build/release/src)
endif()

# Extension source files
set(EXTENSION_SOURCES
    src/pyfunc_extension.cpp
    src/pyfunc_parser.cpp
    src/pyfunc_function.cpp
    src/pyfunc_registry.cpp
)

# Build loadable extension
add_library(${LOADABLE_EXTENSION_NAME} SHARED ${EXTENSION_SOURCES})

target_include_directories(${LOADABLE_EXTENSION_NAME} PRIVATE
    ${CMAKE_SOURCE_DIR}/src/include
    ${DUCKDB_INCLUDE_DIR}
    ${Python3_INCLUDE_DIRS}
)

target_link_libraries(${LOADABLE_EXTENSION_NAME} PRIVATE
    ${DUCKDB_LIBRARY}
    pybind11::embed
    Python3::Python
)

# Set output name without lib prefix
set_target_properties(${LOADABLE_EXTENSION_NAME} PROPERTIES
    OUTPUT_NAME ${TARGET_NAME}
    PREFIX ""
    SUFFIX ".duckdb_extension"
)

# Compiler flags
target_compile_options(${LOADABLE_EXTENSION_NAME} PRIVATE
    -fPIC
    -Wall
    -Wextra
    -Wno-unused-parameter
)

# Platform-specific settings
if(APPLE)
    target_link_options(${LOADABLE_EXTENSION_NAME} PRIVATE -undefined dynamic_lookup)
endif()

# Installation
install(TARGETS ${LOADABLE_EXTENSION_NAME}
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)

# Print configuration summary
message(STATUS "")
message(STATUS "=== PyFunc Extension Configuration ===")
message(STATUS "DuckDB include: ${DUCKDB_INCLUDE_DIR}")
message(STATUS "Python include: ${Python3_INCLUDE_DIRS}")
message(STATUS "Python library: ${Python3_LIBRARIES}")
message(STATUS "======================================")
message(STATUS "")
