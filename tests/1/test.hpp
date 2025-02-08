#ifndef TEST_HPP
#define TEST_HPP

#include <iostream>

class T1 {
public:
    T1() {};
    T1(double d) {};
    virtual ~T1() {};

    T1* createT1() {
        return this;
    };
    void prit(int a) {};
    int add(int a, float b) {
        return a+b;
    };

public:
    int sum;

private:
    int iadd(float a, int b) {
        return a+b;
    };
    float isum;
};

void standalone_empty() {
    
}

double standalone_mutiply(double a, T1* b) {
    return a*b->sum;
}

#endif //TEST_HPP
