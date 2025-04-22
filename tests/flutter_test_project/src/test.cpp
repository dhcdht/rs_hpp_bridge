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

int TestClass::getCount() {
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

// // Map implementations
// std::map<std::string, int> TestClass::getMap() {
//     std::cout << "C++: Creating and returning std::map<std::string, int>." << std::endl;
//     return {{"apple", 1}, {"banana", 2}, {"cherry", 3}};
// }

// void TestClass::processMap(const std::map<std::string, int>& map_data) {
//     std::cout << "C++: Processing std::map<std::string, int>:" << std::endl;
//     for (const auto& pair : map_data) {
//         std::cout << "  {\"" << pair.first << "\": " << pair.second << "}" << std::endl;
//     }
// }
