#include "simple_a.hpp"
#include "simple_b.hpp"
#include <iostream>

SimpleA::SimpleA(int id, std::string name) : id(id), name(name), current_color(Color::RED), current_position(Point()), connected_b(nullptr) {
    std::cout << "SimpleA constructor: " << name << std::endl;
}

SimpleA::~SimpleA() {
    std::cout << "SimpleA destructor: " << name << std::endl;
}

int SimpleA::getId() const {
    return id;
}

std::string SimpleA::getName() const {
    return name;
}

void SimpleA::setName(std::string name) {
    this->name = name;
}

Color SimpleA::getColor() const {
    return current_color;
}

void SimpleA::setColor(Color color) {
    current_color = color;
    std::cout << "SimpleA: Set color to " << static_cast<int>(color) << std::endl;
}

Point SimpleA::getPosition() const {
    return current_position;
}

void SimpleA::setPosition(Point pos) {
    current_position = pos;
    std::cout << "SimpleA: Set position to (" << pos.x << ", " << pos.y << ")" << std::endl;
}

void SimpleA::connectToB(SimpleB* b) {
    connected_b = b;
    std::cout << "SimpleA: Connected to SimpleB" << std::endl;
}

SimpleB* SimpleA::getConnectedB() const {
    return connected_b;
}
