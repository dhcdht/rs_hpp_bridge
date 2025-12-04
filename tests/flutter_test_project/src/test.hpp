#ifndef TEST_HPP
#define TEST_HPP

#include <iostream>
#include <string> // Add include for std::string
#include <vector> // Add include for std::vector
#include <map>      // Add include for std::map
#include <unordered_map> // Add include for std::unordered_map
#include <set>      // Add include for std::set
#include <unordered_set> // Add include for std::unordered_set
#include <memory>   // Add include for std::shared_ptr

// Define a simple struct for testing
struct SimpleStruct {
    int id;
    std::string name;
};

// Define a callback interface (abstract class)
class MyCallback {
public:
    virtual ~MyCallback() = default;
    virtual void onCallback(std::string message) = 0;
    virtual void onGetInt(int value) = 0;
    virtual void onGetStruct(SimpleStruct s)= 0;
    virtual void onGetVector(std::vector<float> v) = 0;
    virtual void onGetConst(const unsigned char* value, size_t size) = 0;
};

class TestClass {
private: // 添加 private 成员变量
    int count = 0; // 新增成员变量 count
    std::shared_ptr<MyCallback> current_callback; // Store the callback

public:
    TestClass();
    virtual ~TestClass();

public:
    double sum(int a, float b);
    // Add new methods for testing
    std::string getString(const std::string str);
    const char* getCharString(const char* str);
    const unsigned char* getUnsignedCharString(const unsigned char* str);
    SimpleStruct getStruct();
    void processStruct(SimpleStruct s);
    static int getStaticValue(const int value);
    std::vector<int> getVector();
    void processVector(std::vector<int> v);

    uint64_t getCount(); // 新增获取 count 的方法
    void incrementCount(); // 新增增加 count 的方法
    std::string getMessage(); // 新增返回 std::string 的方法
    void modifyIntPtr(int* intPtr); // 重命名参数 ptr 为 intPtr
    static std::string getStaticMessage(); // 新增静态方法

    // Callback test
    void registerCallback(std::shared_ptr<MyCallback> callback);
    void triggerCallback(std::string message);
    void triggerGetIntCallback(int value);
    void triggerGetStructCallback(int id, std::string name);
    void triggetGetVectorCallback(std::vector<float> v);
    void triggerGetConstCallback(const unsigned char* value, size_t size);

    // // Overload test
    // void processData(int data);
    // void processData(std::string data);

    // Shared_ptr test
    std::shared_ptr<SimpleStruct> getSharedStruct();
    void processSharedStruct(std::shared_ptr<SimpleStruct> s_ptr);

    // Map and Set test
    std::map<std::string, int> testStdMap(std::map<int, std::string> m);
    std::unordered_map<int, std::string> testStdUnorderedMap(std::unordered_map<std::string, int> m);
    std::set<int> testStdSet(std::set<std::string> s);
    std::unordered_set<std::string> testStdUnorderedSet(std::unordered_set<int> s);

    // Test string-to-string map
    std::map<std::string, std::string> testStdMapStringString(std::map<std::string, std::string> m);
};

#endif // TEST_HPP