#pragma once

#include "duckdb.hpp"
#include "duckdb/parser/parser_extension.hpp"
#include "duckdb/parser/parsed_data/create_scalar_function_info.hpp"
#include "duckdb/function/scalar_function.hpp"
#include "duckdb/catalog/catalog_entry/scalar_function_catalog_entry.hpp"

#include <pybind11/embed.h>
#include <pybind11/stl.h>

#include <string>
#include <vector>
#include <unordered_map>
#include <mutex>
#include <memory>

namespace py = pybind11;

namespace duckdb {

//===--------------------------------------------------------------------===//
// Python Function Definition
//===--------------------------------------------------------------------===//

struct PyFunctionParameter {
    std::string name;
    LogicalType type;
};

struct PyFunctionDefinition {
    std::string name;
    std::vector<PyFunctionParameter> parameters;
    LogicalType return_type;
    std::string body;

    // Cached compiled code object
    mutable py::object compiled_func;
    mutable bool is_compiled = false;
};

//===--------------------------------------------------------------------===//
// Function Registry
//===--------------------------------------------------------------------===//

class PyFunctionRegistry {
public:
    static PyFunctionRegistry& Instance();

    void RegisterFunction(const PyFunctionDefinition& func);
    bool HasFunction(const std::string& name) const;
    const PyFunctionDefinition* GetFunction(const std::string& name) const;
    std::vector<std::string> ListFunctions() const;
    void Clear();

private:
    PyFunctionRegistry() = default;

    mutable std::mutex mutex_;
    std::unordered_map<std::string, PyFunctionDefinition> functions_;
};

//===--------------------------------------------------------------------===//
// Python Executor
//===--------------------------------------------------------------------===//

class PyExecutor {
public:
    static void Initialize();
    static void Finalize();
    static bool IsInitialized();

    static Value Execute(const PyFunctionDefinition& func, const std::vector<Value>& args);

private:
    static bool initialized_;
    static std::unique_ptr<py::scoped_interpreter> interpreter_;
};

//===--------------------------------------------------------------------===//
// Parser Extension
//===--------------------------------------------------------------------===//

class PyFuncParserExtension : public ParserExtension {
public:
    PyFuncParserExtension();

    ParserExtensionParseResult Parse(ParserExtensionInfo *info, const std::string &query) override;

    ParserExtensionPlanResult Plan(ParserExtensionInfo *info, ClientContext &context,
                                   unique_ptr<ParserExtensionParseData> parse_data) override;
};

//===--------------------------------------------------------------------===//
// CREATE FUNCTION Statement
//===--------------------------------------------------------------------===//

struct CreatePyFunctionData : public ParserExtensionParseData {
    PyFunctionDefinition function;

    unique_ptr<ParserExtensionParseData> Copy() const override {
        auto result = make_uniq<CreatePyFunctionData>();
        result->function = function;
        return result;
    }
};

//===--------------------------------------------------------------------===//
// Extension Entry Point
//===--------------------------------------------------------------------===//

class PyFuncExtension : public Extension {
public:
    void Load(DuckDB &db) override;
    std::string Name() override { return "pyfunc"; }
};

//===--------------------------------------------------------------------===//
// Helper Functions
//===--------------------------------------------------------------------===//

LogicalType StringToLogicalType(const std::string& type_str);
std::string LogicalTypeToString(const LogicalType& type);
py::object ValueToPython(const Value& val);
Value PythonToValue(const py::object& obj, const LogicalType& expected_type);

} // namespace duckdb
