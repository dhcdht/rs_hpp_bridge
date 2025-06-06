#include "company.hpp"
#include <iostream>
#include <algorithm>

Company::Company(int id, const std::string& name) 
    : companyId(id), name(name) {
    std::cout << "Company constructor: " << name << std::endl;
}

Company::~Company() {
    std::cout << "Company destructor: " << name << std::endl;
}

int Company::getCompanyId() const {
    return companyId;
}

std::string Company::getName() const {
    return name;
}

void Company::setName(const std::string& name) {
    this->name = name;
}

Address Company::getHeadquarters() const {
    return headquarters;
}

void Company::setHeadquarters(const Address& addr) {
    this->headquarters = addr;
}

void Company::addEmployee(std::shared_ptr<Person> person) {
    if (person) {
        employees.push_back(person);
        person->setCompany(std::make_shared<Company>(*this));
        std::cout << "Added employee: " << person->getName() << std::endl;
    }
}

void Company::removeEmployee(int personId) {
    employees.erase(
        std::remove_if(employees.begin(), employees.end(),
            [personId](const std::shared_ptr<Person>& p) {
                return p && p->getId() == personId;
            }),
        employees.end()
    );
}

std::vector<std::shared_ptr<Person>> Company::getEmployees() const {
    return employees;
}

std::shared_ptr<Person> Company::findEmployeeById(int id) const {
    for (const auto& emp : employees) {
        if (emp && emp->getId() == id) {
            return emp;
        }
    }
    return nullptr;
}

int Company::getEmployeeCount() const {
    return static_cast<int>(employees.size());
}

std::vector<std::shared_ptr<Person>> Company::getEmployeesByType(PersonType type) const {
    std::vector<std::shared_ptr<Person>> result;
    for (const auto& emp : employees) {
        if (emp && emp->getType() == type) {
            result.push_back(emp);
        }
    }
    return result;
}

void Company::notifyCallback(EventCallback* callback) const {
    if (callback) {
        callback->onCompanyCreated(name);
    }
}

std::shared_ptr<Company> Company::createCompany(const std::string& name) {
    return std::make_shared<Company>(1, name);
}

bool Company::transferEmployee(std::shared_ptr<Person> person, std::shared_ptr<Company> targetCompany) {
    if (!person || !targetCompany) {
        return false;
    }
    
    // Remove from current company
    removeEmployee(person->getId());
    
    // Add to target company
    targetCompany->addEmployee(person);
    
    std::cout << "Transferred " << person->getName() << " to " << targetCompany->getName() << std::endl;
    return true;
}
