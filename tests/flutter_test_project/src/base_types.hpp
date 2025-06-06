// Base class with shared types
#ifndef BASE_TYPES_HPP
#define BASE_TYPES_HPP

#include <string>
#include <vector>
#include <memory>

// Forward declarations
class Person;
class Company;

// Shared enums
enum class PersonType {
    EMPLOYEE,
    MANAGER,
    CONTRACTOR
};

// Shared structures
struct Address {
    int id;
    std::string street;
    std::string city;
    std::string country;
};

// Shared callback interface
class EventCallback {
public:
    virtual ~EventCallback() = default;
    virtual void onPersonJoined(Person* person) = 0;
    virtual void onCompanyCreated(const std::string& name) = 0;
};

#endif // BASE_TYPES_HPP
