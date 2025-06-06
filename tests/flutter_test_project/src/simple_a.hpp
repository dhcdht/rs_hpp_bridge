// Simple class A that references class B
#ifndef SIMPLE_A_HPP
#define SIMPLE_A_HPP

#include "simple_types.hpp"

// Forward declaration
class SimpleB;

class SimpleA {
private:
    int id;
    std::string name;
    Color current_color;
    Point current_position;
    SimpleB* connected_b;

public:
    SimpleA(int id, std::string name);
    virtual ~SimpleA();

    // Basic methods
    int getId() const;
    std::string getName() const;
    void setName(std::string name);
    
    // Method that uses types from simple_types.hpp
    Color getColor() const;
    void setColor(Color color);
    
    Point getPosition() const;
    void setPosition(Point pos);
    
    // Method that will reference SimpleB (cross-file reference)
    void connectToB(SimpleB* b);
    SimpleB* getConnectedB() const;
};

#endif // SIMPLE_A_HPP
