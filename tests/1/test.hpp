#ifndef TEST_HPP
#define TEST_HPP

#include <iostream>
#include <string>
#include <vector>

struct S1 {
    int iv1;
    std::string sv1;
    bool varB;
    float varArr[16];
    float* varFp;
    int64_t varI64;
    size_t varSizeT;
    // std::vector<std::string> varVec;
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
    float* floatArr(float arr[16]) {
        return arr;
    }
    void floatPtrPtr(float** a) {
        **a = 10.0f;
    }
    int64_t testInt64T(int64_t a) {
        return a;
    }
    size_t testSizeT(size_t a) {
        return a;
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
        std::cout << "testStruct S1: iv1=" << s.iv1 << ", sv1=" << s.sv1 << std::endl;
        return s;
    }
    std::shared_ptr<S1> testStdPtr(std::shared_ptr<S1> s) {
        std::cout << "testSharedPtr S1: iv1=" << s->iv1 << ", sv1=" << s->sv1 << std::endl;
        return s;
    }
    std::shared_ptr<Callback1> testStdPtrCallback(std::shared_ptr<Callback1> cb) {
        cb->onDoAdd(this->sum, 2.0f);
        cb->onCall(this);
        return cb;
    }
    // std::vector<int> testStdVector(std::vector<std::string> v, std::vector<std::shared_ptr<S1>> v2) {
    //     for (auto& s : v) {
    //         std::cout << s << std::endl;
    //     }
    //     return std::vector<int>();
    // }

public:
    int sum;
    S1* s1p;
    S1 s1;

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
