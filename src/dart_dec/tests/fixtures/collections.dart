// test_cases/collections.dart
// Tests collection literal recovery
void main() {
  // List literal
  final list = [1, 2, 3, 4, 5];

  // Map literal
  final map = {'name': 'Dart', 'version': '3.2', 'year': '2024'};

  // Set literal
  final set = {1, 2, 3, 3, 2, 1};

  // Cascade operator
  final sb = StringBuffer()
    ..write('Hello')
    ..write(' ')
    ..write('World');

  // String interpolation
  final greeting = 'List has ${list.length} items and map has ${map.length} entries';

  // Collection if/for
  final filtered = [for (var i in list) if (i > 2) i * 10];

  print(list);
  print(map);
  print(set);
  print(sb.toString());
  print(greeting);
  print(filtered);
}
