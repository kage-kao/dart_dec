// test_cases/null_safety.dart
// Tests null safety patterns
String? maybeNull(bool flag) {
  if (flag) {
    return "hello";
  }
  return null;
}

int getLength(String? text) {
  return text?.length ?? 0;
}

void main() {
  final result = maybeNull(true);
  print(result!);
  print(getLength(result));
  print(getLength(null));

  late String name;
  name = "Dart";
  print(name);
}
