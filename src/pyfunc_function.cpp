#include "pyfunc_extension.hpp"

#include <sstream>

namespace duckdb {

//===--------------------------------------------------------------------===//
// Python Executor Static Members
//===--------------------------------------------------------------------===//

bool PyExecutor::initialized_ = false;
std::unique_ptr<py::scoped_interpreter> PyExecutor::interpreter_;

//===--------------------------------------------------------------------===//
// Python Executor Implementation
//===--------------------------------------------------------------------===//

void PyExecutor::Initialize() {
    if (initialized_) {
        return;
    }

    // Initialize the Python interpreter
    interpreter_ = std::make_unique<py::scoped_interpreter>();
    initialized_ = true;

    // Import commonly used modules
    py::exec(R"(
import sys
import builtins
    )");
}

void PyExecutor::Finalize() {
    if (!initialized_) {
        return;
    }

    interpreter_.reset();
    initialized_ = false;
}

bool PyExecutor::IsInitialized() {
    return initialized_;
}

Value PyExecutor::Execute(const PyFunctionDefinition& func, const std::vector<Value>& args) {
    if (!initialized_) {
        throw InternalException("Python interpreter not initialized");
    }

    try {
        py::gil_scoped_acquire acquire;

        // Compile the function if not already compiled
        if (!func.is_compiled) {
            // Build the function wrapper code
            std::stringstream code;
            code << "def __pyfunc_wrapper__(";
            for (size_t i = 0; i < func.parameters.size(); i++) {
                if (i > 0) code << ", ";
                code << func.parameters[i].name;
            }
            code << "):\n";

            // Indent the body
            std::istringstream body_stream(func.body);
            std::string line;
            while (std::getline(body_stream, line)) {
                code << "    " << line << "\n";
            }

            // Execute to define the function
            py::exec(code.str());

            // Get the function object
            func.compiled_func = py::globals()["__pyfunc_wrapper__"];
            func.is_compiled = true;
        }

        // Build arguments
        py::tuple py_args(args.size());
        for (size_t i = 0; i < args.size(); i++) {
            py_args[i] = ValueToPython(args[i]);
        }

        // Call the function
        py::object result = func.compiled_func(*py_args);

        // Convert result back to DuckDB Value
        return PythonToValue(result, func.return_type);

    } catch (const py::error_already_set& e) {
        throw InvalidInputException("Python error: %s", e.what());
    }
}

//===--------------------------------------------------------------------===//
// Type Conversion: DuckDB Value -> Python
//===--------------------------------------------------------------------===//

py::object ValueToPython(const Value& val) {
    if (val.IsNull()) {
        return py::none();
    }

    switch (val.type().id()) {
        case LogicalTypeId::BOOLEAN:
            return py::bool_(BooleanValue::Get(val));

        case LogicalTypeId::TINYINT:
            return py::int_(TinyIntValue::Get(val));

        case LogicalTypeId::SMALLINT:
            return py::int_(SmallIntValue::Get(val));

        case LogicalTypeId::INTEGER:
            return py::int_(IntegerValue::Get(val));

        case LogicalTypeId::BIGINT:
            return py::int_(BigIntValue::Get(val));

        case LogicalTypeId::FLOAT:
            return py::float_(FloatValue::Get(val));

        case LogicalTypeId::DOUBLE:
            return py::float_(DoubleValue::Get(val));

        case LogicalTypeId::VARCHAR:
            return py::str(StringValue::Get(val));

        case LogicalTypeId::BLOB: {
            auto blob = StringValue::Get(val);
            return py::bytes(blob.data(), blob.size());
        }

        case LogicalTypeId::DATE: {
            auto date = DateValue::Get(val);
            int32_t year, month, day;
            Date::Convert(date, year, month, day);
            py::object datetime = py::module_::import("datetime");
            return datetime.attr("date")(year, month, day);
        }

        case LogicalTypeId::TIMESTAMP: {
            auto ts = TimestampValue::Get(val);
            date_t date;
            dtime_t time;
            Timestamp::Convert(ts, date, time);
            int32_t year, month, day;
            Date::Convert(date, year, month, day);
            int32_t hour, min, sec, micros;
            Time::Convert(time, hour, min, sec, micros);
            py::object datetime = py::module_::import("datetime");
            return datetime.attr("datetime")(year, month, day, hour, min, sec, micros);
        }

        default:
            // Fallback: convert to string
            return py::str(val.ToString());
    }
}

//===--------------------------------------------------------------------===//
// Type Conversion: Python -> DuckDB Value
//===--------------------------------------------------------------------===//

Value PythonToValue(const py::object& obj, const LogicalType& expected_type) {
    if (obj.is_none()) {
        return Value(expected_type);
    }

    try {
        switch (expected_type.id()) {
            case LogicalTypeId::BOOLEAN:
                return Value::BOOLEAN(obj.cast<bool>());

            case LogicalTypeId::TINYINT:
                return Value::TINYINT(obj.cast<int8_t>());

            case LogicalTypeId::SMALLINT:
                return Value::SMALLINT(obj.cast<int16_t>());

            case LogicalTypeId::INTEGER:
                return Value::INTEGER(obj.cast<int32_t>());

            case LogicalTypeId::BIGINT:
                return Value::BIGINT(obj.cast<int64_t>());

            case LogicalTypeId::FLOAT:
                return Value::FLOAT(obj.cast<float>());

            case LogicalTypeId::DOUBLE:
                return Value::DOUBLE(obj.cast<double>());

            case LogicalTypeId::VARCHAR:
                return Value(py::str(obj).cast<std::string>());

            case LogicalTypeId::BLOB: {
                std::string bytes = obj.cast<std::string>();
                return Value::BLOB(bytes);
            }

            default:
                // Fallback: convert to string
                return Value(py::str(obj).cast<std::string>());
        }
    } catch (const py::cast_error& e) {
        // Try string conversion as fallback
        try {
            return Value(py::str(obj).cast<std::string>());
        } catch (...) {
            throw InvalidInputException("Cannot convert Python object to %s", expected_type.ToString());
        }
    }
}

} // namespace duckdb
