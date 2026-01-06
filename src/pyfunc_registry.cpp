#include "pyfunc_extension.hpp"

namespace duckdb {

//===--------------------------------------------------------------------===//
// PyFunctionRegistry Implementation
//===--------------------------------------------------------------------===//

std::unordered_map<std::string, PyFunctionDefinition> PyFunctionRegistry::functions_;

void PyFunctionRegistry::RegisterFunction(const PyFunctionDefinition& func) {
    functions_[func.name] = func;
}

bool PyFunctionRegistry::HasFunction(const std::string& name) {
    return functions_.find(name) != functions_.end();
}

PyFunctionDefinition& PyFunctionRegistry::GetFunction(const std::string& name) {
    auto it = functions_.find(name);
    if (it == functions_.end()) {
        throw InternalException("Python function '%s' not found", name);
    }
    return it->second;
}

void PyFunctionRegistry::DropFunction(const std::string& name) {
    functions_.erase(name);
}

std::vector<std::string> PyFunctionRegistry::ListFunctions() {
    std::vector<std::string> result;
    result.reserve(functions_.size());
    for (const auto& pair : functions_) {
        result.push_back(pair.first);
    }
    return result;
}

void PyFunctionRegistry::Clear() {
    functions_.clear();
}

} // namespace duckdb
