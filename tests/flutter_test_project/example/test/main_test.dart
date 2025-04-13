import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test_project/flutter_test_project.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('FlutterTestProject Tests', () {
    test('test sum', () async {
      final ret = sum(1, 2);
      expect(ret, 3);
    });
    test('test sumAsync', () async {
      final ret = await sumAsync(2, 3);
      expect(ret, 5);
    });
  });
}


