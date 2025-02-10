#ifndef TEST_HPP
#define TEST_HPP

#include <iostream>

class T1;
class Callback1 {
public:
    Callback1() {};
    virtual ~Callback1() {};

    virtual void onCall(T1* t1) = 0;
    virtual double onDoAdd(int a, float b) = 0;
};

class T1 {
public:
    T1() {};
    T1(double d) {};
    virtual ~T1() {};

    T1* createT1() {
        return this;
    };
    void prit(T1* a) {
        std::cout << a->sum << std::endl;
    };
    int add(int a, float b) {
        return a+b;
    };

    void intArr(int* a, int size) {
        *a = 10;
    }
    void floatPtrPtr(float** a) {
        **a = 10.0f;
    }
    void printString(char* str) {
        std::cout << str << std::endl;
    }

    void setCallback(Callback1* cb) {
        cb->onDoAdd(this->sum, 2.0f);
        cb->onCall(this);
    }

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
