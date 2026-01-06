#include "pyfunc_extension.hpp"
#include "duckdb/parser/parser_extension.hpp"
#include "duckdb/main/client_context.hpp"
#include "duckdb/catalog/catalog.hpp"

#include <regex>
#include <algorithm>
#include <cctype>

namespace duckdb {

//===--------------------------------------------------------------------===//
// Parser Helper Functions
//===--------------------------------------------------------------------===//

static std::string Trim(const std::string& str) {
    size_t start = str.find_first_not_of(" \t\n\r");
    if (start == std::string::npos) return "";
    size_t end = str.find_last_not_of(" \t\n\r");
    return str.substr(start, end - start + 1);
}

static std::string ToUpper(const std::string& str) {
    std::string result = str;
    std::transform(result.begin(), result.end(), result.begin(), ::toupper);
    return result;
}

static bool StartsWith(const std::string& str, const std::string& prefix) {
    return str.size() >= prefix.size() && str.compare(0, prefix.size(), prefix) == 0;
}

//===--------------------------------------------------------------------===//
// CREATE FUNCTION Parser
//===--------------------------------------------------------------------===//

struct CreateFunctionParser {
    const std::string& sql;
    size_t pos = 0;

    CreateFunctionParser(const std::string& query) : sql(query) {}

    bool ParseCreateFunction(PyFunctionDefinition& func) {
        std::string upper = ToUpper(sql);

        // Skip whitespace
        SkipWhitespace();

        // Check for CREATE [OR REPLACE] FUNCTION
        if (!Match("CREATE")) return false;
        SkipWhitespace();

        // Optional OR REPLACE
        if (Match("OR")) {
            SkipWhitespace();
            if (!Match("REPLACE")) return false;
            SkipWhitespace();
        }

        // Must have FUNCTION keyword
        if (!Match("FUNCTION")) return false;
        SkipWhitespace();

        // Parse function name
        func.name = ParseIdentifier();
        if (func.name.empty()) return false;
        SkipWhitespace();

        // Parse parameters
        if (!Expect('(')) return false;
        func.parameters = ParseParameters();
        if (!Expect(')')) return false;
        SkipWhitespace();

        // Parse RETURNS clause
        if (!Match("RETURNS")) return false;
        SkipWhitespace();

        std::string return_type_str = ParseTypeName();
        if (return_type_str.empty()) return false;
        func.return_type = StringToLogicalType(return_type_str);
        SkipWhitespace();

        // Parse LANGUAGE clause
        if (!Match("LANGUAGE")) return false;
        SkipWhitespace();

        std::string language = ParseIdentifier();
        if (ToUpper(language) != "PYTHON") {
            return false; // Only Python supported
        }
        SkipWhitespace();

        // Parse AS clause
        if (!Match("AS")) return false;
        SkipWhitespace();

        // Parse function body (either $$ delimited or single quotes)
        func.body = ParseFunctionBody();
        if (func.body.empty() && pos < sql.size()) {
            // Body might be empty but valid
        }

        return true;
    }

private:
    void SkipWhitespace() {
        while (pos < sql.size() && std::isspace(sql[pos])) {
            pos++;
        }
    }

    bool Match(const std::string& keyword) {
        std::string upper = ToUpper(sql.substr(pos, keyword.size()));
        if (upper == keyword) {
            pos += keyword.size();
            return true;
        }
        return false;
    }

    bool Expect(char c) {
        SkipWhitespace();
        if (pos < sql.size() && sql[pos] == c) {
            pos++;
            return true;
        }
        return false;
    }

    std::string ParseIdentifier() {
        std::string result;
        while (pos < sql.size() && (std::isalnum(sql[pos]) || sql[pos] == '_')) {
            result += sql[pos++];
        }
        return result;
    }

    std::string ParseTypeName() {
        std::string result;
        // Type names can include spaces (e.g., "DOUBLE PRECISION")
        while (pos < sql.size()) {
            char c = sql[pos];
            if (std::isalnum(c) || c == '_') {
                result += c;
                pos++;
            } else if (c == ' ' || c == '\t') {
                // Check if next non-space is part of type
                size_t temp = pos;
                while (temp < sql.size() && std::isspace(sql[temp])) temp++;
                std::string upper_rest = ToUpper(sql.substr(temp));
                // Check for multi-word types or stop
                if (StartsWith(upper_rest, "PRECISION") ||
                    StartsWith(upper_rest, "VARYING") ||
                    StartsWith(upper_rest, "ZONE")) {
                    result += ' ';
                    pos = temp;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return Trim(result);
    }

    std::vector<PyFunctionParameter> ParseParameters() {
        std::vector<PyFunctionParameter> params;
        SkipWhitespace();

        while (pos < sql.size() && sql[pos] != ')') {
            SkipWhitespace();

            // Parse parameter name
            std::string name = ParseIdentifier();
            if (name.empty()) break;

            SkipWhitespace();

            // Parse parameter type
            std::string type_str = ParseTypeName();

            PyFunctionParameter param;
            param.name = name;
            param.type = StringToLogicalType(type_str);
            params.push_back(param);

            SkipWhitespace();

            // Check for comma
            if (pos < sql.size() && sql[pos] == ',') {
                pos++;
                SkipWhitespace();
            }
        }

        return params;
    }

    std::string ParseFunctionBody() {
        SkipWhitespace();

        // Check for $$ delimiter
        if (pos + 1 < sql.size() && sql[pos] == '$' && sql[pos + 1] == '$') {
            pos += 2;
            size_t start = pos;

            // Find closing $$
            while (pos + 1 < sql.size()) {
                if (sql[pos] == '$' && sql[pos + 1] == '$') {
                    std::string body = sql.substr(start, pos - start);
                    pos += 2;
                    return body;
                }
                pos++;
            }
            // No closing delimiter found
            return sql.substr(start);
        }

        // Check for single quotes
        if (pos < sql.size() && sql[pos] == '\'') {
            pos++;
            std::string body;
            while (pos < sql.size()) {
                if (sql[pos] == '\'') {
                    if (pos + 1 < sql.size() && sql[pos + 1] == '\'') {
                        // Escaped quote
                        body += '\'';
                        pos += 2;
                    } else {
                        // End of string
                        pos++;
                        return body;
                    }
                } else {
                    body += sql[pos++];
                }
            }
            return body;
        }

        return "";
    }
};

//===--------------------------------------------------------------------===//
// Parser Extension Implementation
//===--------------------------------------------------------------------===//

PyFuncParserExtension::PyFuncParserExtension() {
}

ParserExtensionParseResult PyFuncParserExtension::Parse(ParserExtensionInfo *info, const std::string &query) {
    std::string trimmed = Trim(query);
    std::string upper = ToUpper(trimmed);

    // Check if this looks like a CREATE FUNCTION ... LANGUAGE PYTHON statement
    if (!StartsWith(upper, "CREATE")) {
        return ParserExtensionParseResult();
    }

    // Check for FUNCTION keyword and LANGUAGE PYTHON
    if (upper.find("FUNCTION") == std::string::npos) {
        return ParserExtensionParseResult();
    }

    if (upper.find("LANGUAGE") == std::string::npos ||
        upper.find("PYTHON") == std::string::npos) {
        return ParserExtensionParseResult();
    }

    // Try to parse as CREATE FUNCTION
    CreateFunctionParser parser(trimmed);
    PyFunctionDefinition func;

    if (!parser.ParseCreateFunction(func)) {
        return ParserExtensionParseResult();
    }

    // Create parse data
    auto parse_data = make_uniq<CreatePyFunctionData>();
    parse_data->function = std::move(func);

    return ParserExtensionParseResult(std::move(parse_data));
}

ParserExtensionPlanResult PyFuncParserExtension::Plan(ParserExtensionInfo *info, ClientContext &context,
                                                       unique_ptr<ParserExtensionParseData> parse_data) {
    auto &create_data = parse_data->Cast<CreatePyFunctionData>();
    auto &func = create_data.function;

    // Register the function in our registry
    PyFunctionRegistry::Instance().RegisterFunction(func);

    // Create the scalar function to register with DuckDB
    ScalarFunctionSet func_set(func.name);

    // Build parameter types
    vector<LogicalType> param_types;
    for (const auto& param : func.parameters) {
        param_types.push_back(param.type);
    }

    // Create the scalar function
    // We need to capture the function name to look it up later
    std::string func_name = func.name;

    auto scalar_func = ScalarFunction(
        param_types,
        func.return_type,
        [func_name](DataChunk &args, ExpressionState &state, Vector &result) {
            const auto* func_def = PyFunctionRegistry::Instance().GetFunction(func_name);
            if (!func_def) {
                throw InternalException("Python function not found: " + func_name);
            }

            idx_t count = args.size();

            // For each row, execute Python function
            for (idx_t i = 0; i < count; i++) {
                std::vector<Value> arg_values;
                for (idx_t col = 0; col < args.ColumnCount(); col++) {
                    arg_values.push_back(args.GetValue(col, i));
                }

                try {
                    Value py_result = PyExecutor::Execute(*func_def, arg_values);
                    result.SetValue(i, py_result);
                } catch (const std::exception& e) {
                    throw InvalidInputException("Python error in function '%s': %s", func_name, e.what());
                }
            }
        }
    );

    func_set.AddFunction(scalar_func);

    // Register with catalog
    CreateScalarFunctionInfo info_obj(func_set);
    info_obj.on_conflict = OnCreateConflict::REPLACE_ON_CONFLICT;

    auto &catalog = Catalog::GetSystemCatalog(context);
    catalog.CreateFunction(context, info_obj);

    // Return success message
    ParserExtensionPlanResult result;
    result.function = make_uniq<ScalarFunction>(
        "pyfunc_result", vector<LogicalType>{}, LogicalType::VARCHAR,
        [func_name](DataChunk &args, ExpressionState &state, Vector &result) {
            result.SetValue(0, Value("Function '" + func_name + "' created successfully"));
        }
    );
    result.requires_valid_transaction = true;
    result.return_type = StatementReturnType::QUERY_RESULT;

    return result;
}

} // namespace duckdb
