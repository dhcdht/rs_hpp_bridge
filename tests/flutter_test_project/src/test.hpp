#ifndef TEST_HPP
#define TEST_HPP

#include <iostream>
#include <string>
#include <vector>
#include <thread>

class TestClass {
public:
    TestClass();
    virtual ~TestClass();

public:
    double sum(int a, float b);
};

#endif // TEST_HPP