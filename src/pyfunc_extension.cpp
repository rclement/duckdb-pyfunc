#define DUCKDB_EXTENSION_MAIN

#include "pyfunc_extension.hpp"
#include "duckdb/main/extension_util.hpp"
#include "duckdb/parser/parser_extension.hpp"

namespace duckdb {

//===--------------------------------------------------------------------===//
// Extension Load
//===--------------------------------------------------------------------===//

void PyFuncExtension::Load(DuckDB &db) {
    // Initialize Python interpreter
    PyExecutor::Initialize();

    // Register parser extension for CREATE FUNCTION syntax
    auto &config = DBConfig::GetConfig(*db.instance);
    config.parser_extensions.push_back(make_uniq<PyFuncParserExtension>());

    // Register utility functions

    // list_py_functions() - table function to list registered functions
    auto list_func = TableFunction("list_py_functions", {}, ListPyFunctionsFunction, ListPyFunctionsBind);
    ExtensionUtil::RegisterFunction(*db.instance, list_func);

    // drop_py_function(name) - scalar function to drop a function
    auto drop_func = ScalarFunction("drop_py_function", {LogicalType::VARCHAR}, LogicalType::VARCHAR,
                                    DropPyFunctionFunction);
    ExtensionUtil::RegisterFunction(*db.instance, drop_func);
}

//===--------------------------------------------------------------------===//
// list_py_functions Table Function
//===--------------------------------------------------------------------===//

struct ListPyFunctionsBindData : public TableFunctionData {
    std::vector<std::string> function_names;
    idx_t current_idx = 0;
};

unique_ptr<FunctionData> ListPyFunctionsBind(ClientContext &context, TableFunctionBindInput &input,
                                              vector<LogicalType> &return_types, vector<string> &names) {
    auto result = make_uniq<ListPyFunctionsBindData>();
    result->function_names = PyFunctionRegistry::Instance().ListFunctions();

    names.push_back("name");
    return_types.push_back(LogicalType::VARCHAR);

    names.push_back("parameters");
    return_types.push_back(LogicalType::VARCHAR);

    names.push_back("return_type");
    return_types.push_back(LogicalType::VARCHAR);

    return std::move(result);
}

void ListPyFunctionsFunction(ClientContext &context, TableFunctionInput &data_p, DataChunk &output) {
    auto &bind_data = data_p.bind_data->CastNoConst<ListPyFunctionsBindData>();

    idx_t count = 0;
    while (bind_data.current_idx < bind_data.function_names.size() && count < STANDARD_VECTOR_SIZE) {
        const auto& name = bind_data.function_names[bind_data.current_idx];
        const auto* func = PyFunctionRegistry::Instance().GetFunction(name);

        if (func) {
            // Name
            output.SetValue(0, count, Value(func->name));

            // Parameters
            std::string params;
            for (size_t i = 0; i < func->parameters.size(); i++) {
                if (i > 0) params += ", ";
                params += func->parameters[i].name + " " + LogicalTypeToString(func->parameters[i].type);
            }
            output.SetValue(1, count, Value(params));

            // Return type
            output.SetValue(2, count, Value(LogicalTypeToString(func->return_type)));

            count++;
        }
        bind_data.current_idx++;
    }

    output.SetCardinality(count);
}

//===--------------------------------------------------------------------===//
// drop_py_function Scalar Function
//===--------------------------------------------------------------------===//

void DropPyFunctionFunction(DataChunk &args, ExpressionState &state, Vector &result) {
    auto &name_vector = args.data[0];

    UnaryExecutor::Execute<string_t, string_t>(name_vector, result, args.size(), [&](string_t name) {
        std::string func_name = name.GetString();
        // Note: For simplicity, we don't actually remove from registry here
        // A full implementation would need proper catalog integration
        return StringVector::AddString(result, "Function '" + func_name + "' drop not yet implemented");
    });
}

//===--------------------------------------------------------------------===//
// Type Conversion Helpers
//===--------------------------------------------------------------------===//

LogicalType StringToLogicalType(const std::string& type_str) {
    std::string upper = type_str;
    std::transform(upper.begin(), upper.end(), upper.begin(), ::toupper);

    // Trim whitespace
    size_t start = upper.find_first_not_of(" \t\n\r");
    size_t end = upper.find_last_not_of(" \t\n\r");
    if (start != std::string::npos && end != std::string::npos) {
        upper = upper.substr(start, end - start + 1);
    }

    if (upper == "VARCHAR" || upper == "STRING" || upper == "TEXT") {
        return LogicalType::VARCHAR;
    } else if (upper == "INTEGER" || upper == "INT" || upper == "INT32") {
        return LogicalType::INTEGER;
    } else if (upper == "BIGINT" || upper == "INT64" || upper == "LONG") {
        return LogicalType::BIGINT;
    } else if (upper == "SMALLINT" || upper == "INT16" || upper == "SHORT") {
        return LogicalType::SMALLINT;
    } else if (upper == "TINYINT" || upper == "INT8") {
        return LogicalType::TINYINT;
    } else if (upper == "FLOAT" || upper == "REAL" || upper == "FLOAT32") {
        return LogicalType::FLOAT;
    } else if (upper == "DOUBLE" || upper == "FLOAT64") {
        return LogicalType::DOUBLE;
    } else if (upper == "BOOLEAN" || upper == "BOOL") {
        return LogicalType::BOOLEAN;
    } else if (upper == "DATE") {
        return LogicalType::DATE;
    } else if (upper == "TIMESTAMP" || upper == "DATETIME") {
        return LogicalType::TIMESTAMP;
    } else if (upper == "BLOB" || upper == "BINARY" || upper == "BYTES") {
        return LogicalType::BLOB;
    }

    // Default to VARCHAR
    return LogicalType::VARCHAR;
}

std::string LogicalTypeToString(const LogicalType& type) {
    return type.ToString();
}

} // namespace duckdb

//===--------------------------------------------------------------------===//
// Extension Entry Points
//===--------------------------------------------------------------------===//

extern "C" {

DUCKDB_EXTENSION_API void pyfunc_init(duckdb::DatabaseInstance &db) {
    duckdb::DuckDB db_wrapper(db);
    db_wrapper.LoadExtension<duckdb::PyFuncExtension>();
}

DUCKDB_EXTENSION_API const char *pyfunc_version() {
    return duckdb::DuckDB::LibraryVersion();
}

}

#ifndef DUCKDB_EXTENSION_MAIN
#error DUCKDB_EXTENSION_MAIN was not defined
#endif
