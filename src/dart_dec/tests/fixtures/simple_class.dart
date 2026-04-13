// test_cases/simple_class.dart
// Compile: dart compile aot-snapshot simple_class.dart
class Animal {
  final String name;
  final int age;

  Animal(this.name, this.age);

  String greet() => "I am $name, age $age";

  bool isOld() {
    return age > 10;
  }
}

void main() {
  final dog = Animal("Rex", 5);
  print(dog.greet());
  print(dog.isOld());
}
