#ifndef TEST_HPP
#define TEST_HPP

#include <iostream>

class T1 {
public:
    T1();
    virtual ~T1();

    T1* createT1();
    void prit(int a);
    int add(int a, float b);

public:
    int sum;

private:
    int iadd(float a, int b);
    float isum;
};

#endif //TEST_HPP
