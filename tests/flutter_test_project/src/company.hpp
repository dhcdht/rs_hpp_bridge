// Company class that references Person and uses base types
#ifndef COMPANY_HPP
#define COMPANY_HPP

#include "base_types.hpp"
#include "person.hpp"
#include <vector>
#include <memory>

class Company {
private:
    int companyId;
    std::string name;
    Address headquarters;
    std::vector<std::shared_ptr<Person>> employees; // Cross-reference to Person

public:
    Company(int id, const std::string& name);
    virtual ~Company();

    // Basic getters/setters
    int getCompanyId() const;
    std::string getName() const;
    void setName(const std::string& name);
    
    Address getHeadquarters() const;
    void setHeadquarters(const Address& addr);
    
    // Employee management with cross-file references
    void addEmployee(std::shared_ptr<Person> person);
    void removeEmployee(int personId);
    std::vector<std::shared_ptr<Person>> getEmployees() const;
    std::shared_ptr<Person> findEmployeeById(int id) const;
    
    // Methods using types from other files
    int getEmployeeCount() const;
    std::vector<std::shared_ptr<Person>> getEmployeesByType(PersonType type) const;
    
    // Callback operations
    void notifyCallback(EventCallback* callback) const;
    
    // Static factory methods
    static std::shared_ptr<Company> createCompany(const std::string& name);
    
    // Complex operations involving multiple types
    bool transferEmployee(std::shared_ptr<Person> person, std::shared_ptr<Company> targetCompany);
};

#endif // COMPANY_HPP
