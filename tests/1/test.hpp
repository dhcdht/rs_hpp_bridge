#ifndef TEST_HPP
#define TEST_HPP

#include <iostream>
#include <string>

struct S1 {
    std::string sv1;
};

class T1;
class Callback1 {
public:
    Callback1() {
        std::cout << "Callback1" << std::endl;
    };
    virtual ~Callback1() {
        std::cout << "~Callback1" << std::endl;
    };

    virtual void onCall(T1* t1) = 0;
    virtual double onDoAdd(int a, float b) = 0;
};

class T1 {
public:
    T1() {
        std::cout << "T1" << std::endl;
    };
    T1(double d) {
        std::cout << "T1_double" << std::endl;
    };
    virtual ~T1() {
        std::cout << "~T1" << std::endl;
    };

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
    void printCharStr(char* str) {
        std::cout << str << std::endl;
    }

    void setCallback(Callback1* cb) {
        cb->onDoAdd(this->sum, 2.0f);
        cb->onCall(this);
    }

    std::string printString(std::string str) {
        std::cout << str << std::endl;
        return "return std::string";
    }
    S1 testStruct(S1 s) {
        std::cout << s.sv1 << std::endl;
        return s;
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
