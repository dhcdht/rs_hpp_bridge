import 'dart:ffi'; // Import dart:ffi

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test_project/flutter_test_project.dart';
import 'package:flutter_test_project/test.dart';
import 'package:flutter_test_project/test_ffiapi.dart';
import 'package:ffi/ffi.dart'; // Import ffi for Utf8 and allocation

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late TestClass testClassInstance; // Declare instance variable

  setUpAll(() async {
    // Initialize FFI
    test_setDylib(flt_dylib);
    // Create a single instance for all tests in this group if needed, or create per test
  });

  // Optional: Create a fresh instance for each test in the group
  setUp(() {
    testClassInstance = TestClass.Constructor();
    expect(testClassInstance, isNotNull);
    print("Dart: New TestClass instance created for test.");
  });

  tearDown(() {
    // Release the C++ object after each test
    testClassInstance.Destructor;
    print("Dart: TestClass instance released.");
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
    test('test Basic Class Methods', () async {
      // Use the instance created in setUp
      final t = testClassInstance;

      // Test sum (already exists)
      final sumResult = t.sum(10, 5.5);
      expect(sumResult, 15.5);

      // Test getCount and incrementCount
      expect(t.getCount(), 0);
      t.incrementCount();
      expect(t.getCount(), 1);
      t.incrementCount();
      expect(t.getCount(), 2);

      // Test getMessage
      final message = t.getMessage();
      expect(message, "Hello from C++ TestClass!");

      // Test modifyIntPtr
      final pointer = calloc<Int64>(); // Allocate memory for an int
      pointer.value = 0;
      t.modifyIntPtr(pointer);
      expect(pointer.value, 12345);
      calloc.free(pointer); // Free the allocated memory

      // Test getStaticMessage (Static methods are called on the class itself)
      final staticMessage = TestClass.getStaticMessage();
      expect(staticMessage, "Hello from C++ static method!");

      // Test getString
      final strResult = t.getString("A string from dart");
      expect(strResult, "A string from dart");

      // Test getStruct and processStruct
      final simpleStruct = t.getStruct();
      expect(simpleStruct.get_id(), 101);
      expect(simpleStruct.get_name(), "StructName");
      // Modify the struct before sending it back (if mutable, otherwise create new)
      // Assuming SimpleStruct is mutable or we create a new one for processStruct
      // Note: Direct modification might not be possible if it's a final class in Dart.
      // Let's assume processStruct just prints for now.
      t.processStruct(simpleStruct); // Check C++ console output

      // Test getStaticValue (Static method)
      final staticValue = TestClass.getStaticValue(42);
      expect(staticValue, 42);

      // Test getVector and processVector
      final vec = t.getVector();
      // expect(vec, orderedEquals([1, 2, 3, 4, 5]));
      expect(vec.at(0), 1);
      expect(vec.at(1), 2);
      expect(vec.at(2), 3);
      expect(vec.at(3), 4);
      expect(vec.at(4), 5);
      t.processVector(vec); // Check C++ console output
    });

    test('test Callback', () async {

      // Initialize the Dart API
      final _initResult = ffi_Dart_InitializeApiDL(NativeApi.initializeApiDLData);
      if (_initResult != 0) {
        throw Exception('Failed to initialize Dart API: $_initResult');
      }

      final t = testClassInstance;
      final callbackImpl = MyCallback.Constructor();
      var callbackImpl_onCallback_called = false;
      var callbackImpl_onCallback_message = "";
      var callbackImpl_onGetInt_value = 0;
      callbackImpl.onCallback_block= (message) {
        print("Dart: Callback received message: $message");
        callbackImpl_onCallback_called = true;
        callbackImpl_onCallback_message = message;
      };
      callbackImpl.onGetInt_block = (value) {
        print("Dart: Callback received int: $value");
        callbackImpl_onGetInt_value = value;
      };
      final callbackPtr = StdPtr_MyCallback.Constructor(callbackImpl);

      // Register the callback
      // IMPORTANT: Need to ensure callbackImpl stays alive.
      // For testing, keeping it in scope might be enough.
      // In real apps, manage lifetime carefully.
      t.registerCallback(callbackPtr);

      // Trigger the string callback
      final testMessage = "test callback message";
      t.triggerCallback(testMessage);
      // Allow some time for async callback if needed, though this FFI call might be sync
      await Future.delayed(Duration(milliseconds: 100));
      expect(callbackImpl_onCallback_called, isTrue);
      expect(callbackImpl_onCallback_message, testMessage);

      // Trigger the int callback
      final testInt = 999;
      t.triggerGetIntCallback(testInt);
      await Future.delayed(Duration(milliseconds: 100));
      expect(callbackImpl_onGetInt_value, testInt);
      // We can't directly check the return value received by C++ here,
      // but we verified the Dart method was called and returned the expected value.
      // Check C++ console output for "Got int from callback: 999"
    });

    // test('test Overload', () async {
    //   final t = testClassInstance;
    //   // Call the int version
    //   t.processData_1(123); // Assuming generated name is processData_1 for int
    //   // Call the string version
    //   t.processData_2("Test String Data"); // Assuming generated name is processData_2 for string
    //   // Check C++ console output for "Processing int data: 123"
    //   // Check C++ console output for "Processing string data: Test String Data"
    // });

    test('test SharedPtr', () async {
      final t = testClassInstance;

      // Test getSharedStruct
      final sharedStructPtr = t.getSharedStruct();
      expect(sharedStructPtr, isNotNull);
      // Accessing fields might require dereferencing or specific generated methods
      // Assuming direct access or getter methods are generated:
      // expect(sharedStructPtr.ref.id, 202); // Example if it's a Pointer<SimpleStruct>
      // expect(sharedStructPtr.ref.name.toDartString(), "SharedStructName"); // Example
      // OR if the generated code returns a Dart class instance directly:
      expect(sharedStructPtr.get().get_id(), 202);
      expect(sharedStructPtr.get().get_name(), "SharedStructName");

      // Test processSharedStruct
      t.processSharedStruct(sharedStructPtr); // Pass the obtained shared_ptr back
      // Check C++ console output for "Processing shared_ptr<SimpleStruct>: id=202, name=SharedStructName"

      // Release the shared ptr if necessary (depends on generation logic)
      // sharedStructPtr.release(); // Or similar method if generated
    });

    // test('test Map', () async {
    //   final t = testClassInstance;

    //   // Test getMap
    //   final mapData = t.getMap();
    //   expect(mapData, isA<Map<String, int>>());
    //   expect(mapData.length, 3);
    //   expect(mapData['apple'], 1);
    //   expect(mapData['banana'], 2);
    //   expect(mapData['cherry'], 3);

    //   // Test processMap
    //   final mapToSend = {'one': 10, 'two': 20};
    //   t.processMap(mapToSend);
    //   // Check C++ console output for "Processing std::map<std::string, int>:"
    //   // and the key-value pairs.
    // });
  });
}


