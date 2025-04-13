import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test_project/flutter_test_project.dart';
import 'package:flutter_test_project/test.dart';
import 'package:flutter_test_project/test_ffiapi.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async {
    // Initialize any resources needed for the tests
    test_setDylib(flt_dylib);
  });

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

  group('RsHppBridge Tests', () {
    test('test Class', () async {
      final t = TestClass.Constructor();
      expect(t, isNotNull);

      final sum = t.sum(1, 2);
      expect(sum, 3);
    });
  });
}


