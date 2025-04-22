#ifndef TEST_HPP
#define TEST_HPP

#include <iostream>
#include <string> // Add include for std::string
#include <vector> // Add include for std::vector
#include <map>      // Add include for std::map
#include <memory>   // Add include for std::shared_ptr

// Define a simple struct for testing
struct SimpleStruct {
    int id;
    std::string name;
};

// // Define a callback interface (abstract class)
// class MyCallback {
// public:
//     virtual ~MyCallback() = default;
//     virtual void onCallback(std::string message) = 0;
//     virtual int onGetInt() = 0;
// };

class TestClass {
private: // 添加 private 成员变量
    int count = 0; // 新增成员变量 count
    // std::shared_ptr<MyCallback> current_callback; // Store the callback

public:
    TestClass();
    virtual ~TestClass();

public:
    double sum(int a, float b);
    // Add new methods for testing
    std::string getString(const std::string str);
    SimpleStruct getStruct();
    void processStruct(SimpleStruct s);
    static int getStaticValue(const int value);
    std::vector<int> getVector();
    void processVector(std::vector<int> v);

    int getCount(); // 新增获取 count 的方法
    void incrementCount(); // 新增增加 count 的方法
    std::string getMessage(); // 新增返回 std::string 的方法
    void modifyIntPtr(int* intPtr); // 重命名参数 ptr 为 intPtr
    static std::string getStaticMessage(); // 新增静态方法

    // // Callback test
    // void registerCallback(std::shared_ptr<MyCallback> callback);
    // void triggerCallback(std::string message);
    // void triggerGetIntCallback();

    // // Overload test
    // void processData(int data);
    // void processData(std::string data);

    // Shared_ptr test
    std::shared_ptr<SimpleStruct> getSharedStruct();
    void processSharedStruct(std::shared_ptr<SimpleStruct> s_ptr);

    // // Map test
    // std::map<std::string, int> getMap();
    // void processMap(const std::map<std::string, int>& map_data);
};

#endif // TEST_HPP