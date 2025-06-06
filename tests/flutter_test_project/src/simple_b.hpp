// Simple class B that references class A
#ifndef SIMPLE_B_HPP
#define SIMPLE_B_HPP

#include "simple_types.hpp"
#include "simple_a.hpp"

class SimpleB {
private:
    int value;
    Color color;
    SimpleA* connected_a;

public:
    SimpleB(int value);
    virtual ~SimpleB();

    // Basic methods
    int getValue() const;
    void setValue(int value);
    
    // Method that uses types from simple_types.hpp
    Color getColor() const;
    void setColor(Color color);
    
    // Method that references SimpleA (cross-file reference)
    void connectToA(SimpleA* a);
    SimpleA* getConnectedA() const;
    
    // Method that uses both local and cross-file types
    Point processWithA(SimpleA* a, Point input);
};

#endif // SIMPLE_B_HPP
