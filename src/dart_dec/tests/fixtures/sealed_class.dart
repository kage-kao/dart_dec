// test_cases/sealed_class.dart
// Tests Dart 3.x sealed classes and exhaustive switch
sealed class Shape {}
class Circle extends Shape {
  final double radius;
  Circle(this.radius);
}
class Square extends Shape {
  final double side;
  Square(this.side);
}
class Triangle extends Shape {
  final double base;
  final double height;
  Triangle(this.base, this.height);
}

double area(Shape s) => switch (s) {
  Circle(radius: var r) => 3.14159 * r * r,
  Square(side: var s) => s * s,
  Triangle(base: var b, height: var h) => 0.5 * b * h,
};

// Records
(int, String) makeRecord() => (42, "hello");

void main() {
  print(area(Circle(5.0)));
  print(area(Square(3.0)));
  print(area(Triangle(4.0, 6.0)));

  final (num, text) = makeRecord();
  print('$num: $text');
}
