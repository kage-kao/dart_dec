// test_cases/async_function.dart
// Tests async/await state machine recovery
import 'dart:async';

Future<int> fetchA() async {
  await Future.delayed(Duration(seconds: 1));
  return 42;
}

Future<int> fetchB() async {
  await Future.delayed(Duration(seconds: 1));
  return 58;
}

Future<int> compute() async {
  final a = await fetchA();
  final b = await fetchB();
  return a + b;
}

Stream<int> countStream() async* {
  for (int i = 0; i < 10; i++) {
    await Future.delayed(Duration(milliseconds: 100));
    yield i;
  }
}

void main() async {
  final result = await compute();
  print('Result: $result');

  await for (final value in countStream()) {
    print('Value: $value');
  }
}
