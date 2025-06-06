#include "person.hpp"
#include "company.hpp"
#include <iostream>

Person::Person(int id, const std::string& name, PersonType type) 
    : id(id), name(name), type(type) {
    std::cout << "Person constructor: " << name << std::endl;
}

Person::~Person() {
    std::cout << "Person destructor: " << name << std::endl;
}

int Person::getId() const {
    return id;
}

std::string Person::getName() const {
    return name;
}

void Person::setName(const std::string& name) {
    this->name = name;
}

PersonType Person::getType() const {
    return type;
}

void Person::setType(PersonType type) {
    this->type = type;
}

Address Person::getAddress() const {
    return address;
}

void Person::setAddress(const Address& addr) {
    this->address = addr;
}

std::shared_ptr<Company> Person::getCompany() const {
    return company;
}

void Person::setCompany(std::shared_ptr<Company> comp) {
    this->company = comp;
}

void Person::processWithCallback(EventCallback* callback) {
    if (callback) {
        callback->onPersonJoined(this);
    }
}

Person* Person::createEmployee(const std::string& name) {
    return new Person(1, name, PersonType::EMPLOYEE);
}

Person* Person::createManager(const std::string& name) {
    return new Person(2, name, PersonType::MANAGER);
}

std::vector<std::string> Person::getSkills() const {
    // Return dummy skills for testing
    return {"C++", "Dart", "Flutter"};
}

void Person::addSkill(const std::string& skill) {
    std::cout << "Adding skill: " << skill << " to " << name << std::endl;
}

void Person::setSkills(const std::vector<std::string>& skills) {
    std::cout << "Setting " << skills.size() << " skills for " << name << std::endl;
}
