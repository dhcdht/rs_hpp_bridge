// Person class that references Company and uses base types
#ifndef PERSON_HPP
#define PERSON_HPP

#include "base_types.hpp"
#include <memory>

// Forward declaration
class Company;

class Person {
private:
    int id;
    std::string name;
    PersonType type;
    Address address;
    std::shared_ptr<Company> company; // Cross-reference to Company

public:
    Person(int id, const std::string& name, PersonType type);
    virtual ~Person();

    // Basic getters/setters
    int getId() const;
    std::string getName() const;
    void setName(const std::string& name);
    
    PersonType getType() const;
    void setType(PersonType type);
    
    Address getAddress() const;
    void setAddress(const Address& addr);
    
    // Cross-file references
    std::shared_ptr<Company> getCompany() const;
    void setCompany(std::shared_ptr<Company> comp);
    
    // Method that uses types from other files
    void processWithCallback(EventCallback* callback);
    
    // Static methods
    static Person* createEmployee(const std::string& name);
    static Person* createManager(const std::string& name);
    
    // Vector operations
    std::vector<std::string> getSkills() const;
    void addSkill(const std::string& skill);
    void setSkills(const std::vector<std::string>& skills);
};

#endif // PERSON_HPP
