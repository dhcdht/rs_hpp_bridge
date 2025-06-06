#include "simple_b.hpp"
#include "simple_a.hpp"
#include <iostream>

SimpleB::SimpleB(int value) : value(value), color(Color::BLUE), connected_a(nullptr) {
    std::cout << "SimpleB constructor: " << value << std::endl;
}

SimpleB::~SimpleB() {
    std::cout << "SimpleB destructor: " << value << std::endl;
}

int SimpleB::getValue() const {
    return value;
}

void SimpleB::setValue(int value) {
    this->value = value;
}

Color SimpleB::getColor() const {
    return color;
}

void SimpleB::setColor(Color color) {
    this->color = color;
    std::cout << "SimpleB: Set color to " << static_cast<int>(color) << std::endl;
}

void SimpleB::connectToA(SimpleA* a) {
    connected_a = a;
    std::cout << "SimpleB: Connected to SimpleA" << std::endl;
}

SimpleA* SimpleB::getConnectedA() const {
    return connected_a;
}

Point SimpleB::processWithA(SimpleA* a, Point input) {
    std::cout << "SimpleB: Processing with SimpleA, input Point(" << input.x << ", " << input.y << ")" << std::endl;
    Point result;
    result.x = input.x + a->getId();
    result.y = input.y + getValue();
    return result;
}
