// Simple shared types
#ifndef SIMPLE_TYPES_HPP
#define SIMPLE_TYPES_HPP

#include <string>

// Forward declaration
class SimpleB;

// Simple enum
enum class Color {
    RED,
    GREEN,
    BLUE
};

// Simple struct
struct Point {
    int x;
    int y;
    
    // Default constructor to ensure proper initialization
    Point() : x(0), y(0) {}
    Point(int x, int y) : x(x), y(y) {}
};

#endif // SIMPLE_TYPES_HPP
