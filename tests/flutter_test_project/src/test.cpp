#include "test.hpp"
#include <iostream> // Add include for std::cout
#include <vector>   // Add include for std::vector
#include <string>   // Add include for std::string

TestClass::TestClass() {
    std::cout << "TestClass Constructor called" << std::endl;
}

TestClass::~TestClass() {
    std::cout << "TestClass Destructor called" << std::endl;
}

double TestClass::sum(int a, float b) {
    return a + b;
}

uint64_t TestClass::getCount() {
    return count; 
}

void TestClass::incrementCount() {
    count++;
}

std::string TestClass::getMessage() {
    return "Hello from C++ TestClass!";
}

void TestClass::modifyIntPtr(int* intPtr) { // 重命名参数 ptr 为 intPtr
    if (intPtr != nullptr) {
        *intPtr = 12345; // 修改指针指向的值
    }
}

std::string TestClass::getStaticMessage() {
    return "Hello from C++ static method!";
}

std::string TestClass::getString(const std::string str) {
    return str;
}

const char* TestClass::getCharString(const char* str) {
    return str;
}

const unsigned char* TestClass::getUnsignedCharString(const unsigned char* str) {
    return str;
}

SimpleStruct TestClass::getStruct() {
    return SimpleStruct{101, "StructName"};
}

void TestClass::processStruct(SimpleStruct s) {
    std::cout << "Processing struct in C++: id=" << s.id << ", name=" << s.name << std::endl;
}

int TestClass::getStaticValue(const int value) {
    return value;
}

std::vector<int> TestClass::getVector() {
    return {1, 2, 3, 4, 5};
}

void TestClass::processVector(std::vector<int> v) {
    std::cout << "Processing vector in C++: ";
    for (int val : v) {
        std::cout << val << " ";
    }
    std::cout << std::endl;
}

// Callback implementations
void TestClass::registerCallback(std::shared_ptr<MyCallback> callback) {
    current_callback = callback;
    std::cout << "C++: Callback registered." << std::endl;
}

void TestClass::triggerCallback(std::string message) {
    if (current_callback) {
        std::cout << "C++: Triggering callback with message: " << message << std::endl;
        current_callback->onCallback(message);
    } else {
        std::cout << "C++: No callback registered." << std::endl;
    }
}

void TestClass::triggerGetIntCallback(int value) {
    if (current_callback) {
        std::cout << "C++: Triggering getInt callback." << std::endl;
        current_callback->onGetInt(value);
    } else {
        std::cout << "C++: No callback registered for getInt." << std::endl;
    }
}

void TestClass::triggerGetStructCallback(int id, std::string name) {
    if (current_callback) {
        std::cout << "C++: Triggering onGetStruct callback." << std::endl;
        SimpleStruct s = { id, name };
        current_callback->onGetStruct(s);
    } else {
        std::cout << "C++: No callback registered for onGetStruct." << std::endl;
    }
}

void TestClass::triggetGetVectorCallback(std::vector<float> v) {
    if (current_callback) {
        std::cout << "C++: Triggering onGetVector callback." << std::endl;
        current_callback->onGetVector(v);
    } else {
        std::cout << "C++: No callback registered for onGetVector." << std::endl;
    }
}

void TestClass::triggerGetConstCallback(const unsigned char* value, size_t size) {
    if (current_callback) {
        std::cout << "C++: Triggering onGetConst callback." << std::endl;
        current_callback->onGetConst(value, size);
    } else {
        std::cout << "C++: No callback registered for onGetConst." << std::endl;
    }
}

// // Overload implementations
// void TestClass::processData(int data) {
//     std::cout << "C++: Processing int data: " << data << std::endl;
// }

// void TestClass::processData(std::string data) {
//     std::cout << "C++: Processing string data: " << data << std::endl;
// }

// Shared_ptr implementations
std::shared_ptr<SimpleStruct> TestClass::getSharedStruct() {
    std::cout << "C++: Creating and returning shared_ptr<SimpleStruct>." << std::endl;
    return std::make_shared<SimpleStruct>(SimpleStruct{202, "SharedStructName"});
}

void TestClass::processSharedStruct(std::shared_ptr<SimpleStruct> s_ptr) {
    if (s_ptr) {
        std::cout << "C++: Processing shared_ptr<SimpleStruct>: id=" << s_ptr->id << ", name=" << s_ptr->name << std::endl;
    } else {
        std::cout << "C++: Received null shared_ptr<SimpleStruct>." << std::endl;
    }
}

// Map and Set implementations
std::map<std::string, int> TestClass::testStdMap(std::map<int, std::string> m) {
    std::cout << "C++: Testing std::map<int, std::string> -> std::map<std::string, int>" << std::endl;
    std::map<std::string, int> result;
    for (const auto& pair : m) {
        result[pair.second] = pair.first;
    }
    return result;
}

std::unordered_map<int, std::string> TestClass::testStdUnorderedMap(std::unordered_map<std::string, int> m) {
    std::cout << "C++: Testing std::unordered_map<std::string, int> -> std::unordered_map<int, std::string>" << std::endl;
    std::unordered_map<int, std::string> result;
    for (const auto& pair : m) {
        result[pair.second] = pair.first;
    }
    return result;
}

std::set<int> TestClass::testStdSet(std::set<std::string> s) {
    std::cout << "C++: Testing std::set<std::string> -> std::set<int>" << std::endl;
    std::set<int> result;
    for (const auto& str : s) {
        result.insert(static_cast<int>(str.length()));
    }
    return result;
}

std::unordered_set<std::string> TestClass::testStdUnorderedSet(std::unordered_set<int> s) {
    std::cout << "C++: Testing std::unordered_set<int> -> std::unordered_set<std::string>" << std::endl;
    std::unordered_set<std::string> result;
    for (const auto& num : s) {
        result.insert(std::to_string(num));
    }
    return result;
}

std::map<std::string, std::string> TestClass::testStdMapStringString(std::map<std::string, std::string> m) {
    std::cout << "C++: Testing std::map<std::string, std::string>" << std::endl;
    std::map<std::string, std::string> result;
    for (const auto& pair : m) {
        result[pair.first + "_modified"] = pair.second + "_modified";
    }
    return result;
}

// Test callback methods with return values
int TestClass::testCallbackComputeSum(int a, int b) {
    std::cout << "C++: Testing callback onComputeSum(" << a << ", " << b << ")" << std::endl;
    if (current_callback) {
        std::cout << "C++: About to call current_callback->onComputeSum" << std::endl;
        std::cout.flush();
        int result = current_callback->onComputeSum(a, b);
        std::cout << "C++: Callback returned: " << result << std::endl;
        return result;
    }
    std::cout << "C++: No callback registered" << std::endl;
    return 0;
}

double TestClass::testCallbackComputeAverage(double x, double y) {
    std::cout << "C++: Testing callback onComputeAverage(" << x << ", " << y << ")" << std::endl;
    if (current_callback) {
        double result = current_callback->onComputeAverage(x, y);
        std::cout << "C++: Callback returned: " << result << std::endl;
        return result;
    }
    std::cout << "C++: No callback registered" << std::endl;
    return 0.0;
}

bool TestClass::testCallbackShouldContinue() {
    std::cout << "C++: Testing callback onShouldContinue()" << std::endl;
    if (current_callback) {
        bool result = current_callback->onShouldContinue();
        std::cout << "C++: Callback returned: " << (result ? "true" : "false") << std::endl;
        return result;
    }
    std::cout << "C++: No callback registered" << std::endl;
    return false;
}

void TestClass::testCallbackLogMessage(std::string message) {
    std::cout << "C++: Testing sync callback onLogMessage with message: " << message << std::endl;
    if (current_callback) {
        current_callback->onLogMessage(message);
        std::cout << "C++: Sync void callback completed" << std::endl;
    } else {
        std::cout << "C++: No callback registered" << std::endl;
    }
}

int TestClass::testCallbackCalculateAsync(int x, int y) {
    std::cout << "C++: Testing async callback onCalculateAsync(" << x << ", " << y << ")" << std::endl;
    if (current_callback) {
        int result = current_callback->onCalculateAsync(x, y);
        std::cout << "C++: Async callback returned: " << result << std::endl;
        return result;
    }
    std::cout << "C++: No callback registered" << std::endl;
    return 0;
}
