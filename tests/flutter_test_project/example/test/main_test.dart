import 'dart:ffi'; // Import dart:ffi

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test_project/flutter_test_project.dart';
import 'package:flutter_test_project/output/test.dart';
import 'package:flutter_test_project/output/test_ffiapi.dart';
import 'package:ffi/ffi.dart'; // Import ffi for Utf8 and allocation
import 'package:flutter_test_project/output/TestModule_public.dart';

// Multi-file imports - test all generated bindings
import 'package:flutter_test_project/output/simple_types.dart';
import 'package:flutter_test_project/output/simple_types_ffiapi.dart';
import 'package:flutter_test_project/output/simple_a.dart';
import 'package:flutter_test_project/output/simple_a_ffiapi.dart';
import 'package:flutter_test_project/output/simple_b.dart';
import 'package:flutter_test_project/output/simple_b_ffiapi.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late TestClass testClassInstance; // Declare instance variable

  setUpAll(() async {
    // Initialize FFI
    TestModule_setDylib(flt_dylib);
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

      // test getCharString
      final charString = "A char string from dart".toNativeUtf8();
      final charStringResult = t.getCharString(charString.cast());
      // expect(charStringResult.elementAt(0).toString().codeUnitAt(0), 'A'.codeUnitAt(0));

      // test getUnsignedCharString
      final unsignedCharString = "A unsigned char string from dart".toNativeUtf8();
      final unsignedCharStringResult = t.getUnsignedCharString(unsignedCharString.cast());
      // expect(unsignedCharStringResult.elementAt(0).toString().codeUnitAt(0), 'A'.codeUnitAt(0));

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

      // Trigger the struct callback
      SimpleStruct? callbackImpl_onGetStruct_value = null;
      callbackImpl.onGetStruct_block =(s) {
        callbackImpl_onGetStruct_value = s;
      };
      final testStruct = SimpleStruct.Constructor();
      t.triggerGetStructCallback(10240, "st");
      await Future.delayed(Duration(milliseconds: 100));
      expect(callbackImpl_onGetStruct_value?.get_id(), 10240);
      expect(callbackImpl_onGetStruct_value?.get_name(), "st");

      // Trigger the vector callback
      StdVector_float? callbackImpl_onGetVector_value = null;
      callbackImpl.onGetVector_block = (v) {
        callbackImpl_onGetVector_value = v;
      };
      StdVector_float vf = StdVector_float.Constructor();
      t.triggetGetVectorCallback(vf);
      await Future.delayed(Duration(milliseconds: 100));
      expect(callbackImpl_onGetVector_value?.size(), 0);

      // Trigger the const callback
      Pointer<Int8>? callbackImpl_onGetConst_value = null;
      int callbackImpl_onGetConst_size = 0;
      callbackImpl.onGetConst_block = (v, size) {
        callbackImpl_onGetConst_value = v;
        callbackImpl_onGetConst_size = size;
      };
      final testConst = "A const cahr from dart";
      final testConstLength = testConst.toNativeUtf8().length;
      t.triggerGetConstCallback(testConst.toNativeUtf8().cast(), testConstLength);
      await Future.delayed(Duration(milliseconds: 100));
      expect(callbackImpl_onGetConst_value?.elementAt(0).value, 'A'.codeUnitAt(0));
      expect(callbackImpl_onGetConst_value?.elementAt(testConstLength-1).value, 't'.codeUnitAt(0));
      expect(callbackImpl_onGetConst_size, testConst.toNativeUtf8().length);

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

    test('test StdMap', () async {
      final t = TestClass.Constructor();
      
      // Create a Map<int, String> to send to C++
      final inputMap = <int, String>{
        1: "One",
        2: "Two",
        3: "Three"
      };
      
      // Convert to StdMap using convenience constructor
      final stdInputMap = StdMap_int_String.fromMap(inputMap);
      
      // Test sending StdMap to C++ and getting result back
      final resultMap = t.testStdMap(stdInputMap);
      expect(resultMap, isA<StdMap_String_int>());
      expect(resultMap.length, 3);
      expect(resultMap["One"], 1);
      expect(resultMap["Two"], 2);
      expect(resultMap["Three"], 3);
      expect(resultMap.contains("One"), true);
      expect(resultMap.contains("Four"), false);
    });

    test('test StdUnorderedMap', () async {
      final t = TestClass.Constructor();
      
      // Create a Map<String, int> to send to C++
      final inputMap = <String, int>{
        "One": 1,
        "Two": 2,
        "Three": 3
      };
      
      // Convert to StdUnorderedMap using convenience constructor
      final stdInputMap = StdUnorderedMap_String_int.fromMap(inputMap);
      
      // Test sending StdUnorderedMap to C++ and getting result back
      final resultMap = t.testStdUnorderedMap(stdInputMap);
      expect(resultMap, isA<StdUnorderedMap_int_String>());
      expect(resultMap.length, 3);
      expect(resultMap[1], "One");
      expect(resultMap[2], "Two");
      expect(resultMap[3], "Three");
      expect(resultMap.contains(1), true);
      expect(resultMap.contains(4), false);
    });

    test('test StdSet', () async {
      final t = TestClass.Constructor();
      
      // Create a Set<String> to send to C++
      final inputSet = <String>{"Apple", "Banana", "Cherry"};
      
      // Convert to StdSet using convenience constructor
      final stdInputSet = StdSet_String.fromSet(inputSet);
      
      // Test sending StdSet to C++ and getting result back
      final resultSet = t.testStdSet(stdInputSet);
      expect(resultSet, isA<StdSet_int>());
      // Set should contain lengths of strings
      expect(resultSet.contains(5), true); // "Apple" length
      expect(resultSet.contains(6), true); // "Banana" length
      expect(resultSet.contains(6), true); // "Cherry" length
    });

    test('test StdUnorderedSet', () async {
      final t = TestClass.Constructor();
      
      // Create a Set<int> to send to C++
      final inputSet = <int>{10, 20, 30};
      
      // Convert to StdUnorderedSet using convenience constructor
      final stdInputSet = StdUnorderedSet_int.fromSet(inputSet);
      
      // Test sending StdUnorderedSet to C++ and getting result back
      final resultSet = t.testStdUnorderedSet(stdInputSet);
      expect(resultSet, isA<StdUnorderedSet_String>());
      // Set should contain string representations of numbers
      expect(resultSet.contains("10"), true);
      expect(resultSet.contains("20"), true);
      expect(resultSet.contains("30"), true);
    });
  });

  group('Multi-file Bridge Tests', () {
    test('test SimpleTypes (Point)', () async {
      // Test Point struct
      final point = Point.Constructor();
      
      // Test setting and getting values
      point.set_x(10);
      point.set_y(20);
      expect(point.get_x(), 10);
      expect(point.get_y(), 20);
    });

    test('test SimpleA class', () async {
      // Test SimpleA constructor and basic methods
      final simpleA = SimpleA.Constructor_int_String(1, "TestA");

      expect(simpleA.getId(), 1);
      expect(simpleA.getName(), "TestA");

      // Test name setter
      simpleA.setName("UpdatedA");
      expect(simpleA.getName(), "UpdatedA");

      // Test Point interaction
      final point = Point.Constructor();
      point.set_x(100);
      point.set_y(200);
      simpleA.setPosition(point);

      final retrievedPoint = simpleA.getPosition();
      expect(retrievedPoint.get_x(), 100);
      expect(retrievedPoint.get_y(), 200);

      // Test Color enum
      simpleA.setColor(Color.red.value);
      expect(simpleA.getColor(), Color.red.value);

      simpleA.setColor(Color.green.value);
      expect(simpleA.getColor(), Color.green.value);

      simpleA.setColor(Color.blue.value);
      expect(simpleA.getColor(), Color.blue.value);

      // Test fromValue conversion
      final colorValue = simpleA.getColor();
      final color = Color.fromValue(colorValue);
      expect(color, Color.blue);
    });

    test('test SimpleB class', () async {
      // Test SimpleB constructor and basic methods
      final simpleB = SimpleB.Constructor_int(42);
      
      expect(simpleB.getValue(), 42);
      
      // Test value setter
      simpleB.setValue(84);
      expect(simpleB.getValue(), 84);
    });

    test('test Cross-file References (SimpleA <-> SimpleB)', () async {
      // Create instances of both classes
      final simpleA = SimpleA.Constructor_int_String(1, "A1");
      final simpleB = SimpleB.Constructor_int(100);
      
      // Test cross-references
      simpleA.connectToB(simpleB);
      simpleB.connectToA(simpleA);
      
      // Verify connections
      final connectedB = simpleA.getConnectedB();
      final connectedA = simpleB.getConnectedA();
      
      expect(connectedB, isNotNull);
      expect(connectedA, isNotNull);
      expect(connectedB.getValue(), 100);
      expect(connectedA.getId(), 1);
      
      // Test complex cross-file method
      final inputPoint = Point.Constructor();
      inputPoint.set_x(10);
      inputPoint.set_y(20);
      
      final resultPoint = simpleB.processWithA(simpleA, inputPoint);
      expect(resultPoint, isNotNull);
    });
  });
}


