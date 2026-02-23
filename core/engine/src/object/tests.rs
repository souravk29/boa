use crate::{JsNativeErrorKind, TestAction, run_test_actions};
use indoc::indoc;

#[test]
fn ordinary_has_instance_nonobject_prototype() {
    run_test_actions([TestAction::assert_native_error(
        indoc! {r#"
            function C() {}
            C.prototype = 1
            String instanceof C
        "#},
        JsNativeErrorKind::Type,
        "function has non-object prototype in instanceof check",
    )]);
}

#[test]
fn object_properties_return_order() {
    run_test_actions([
        TestAction::run_harness(),
        TestAction::run(indoc! {r#"
                var o = {
                    p1: 'v1',
                    p2: 'v2',
                    p3: 'v3',
                };
                o.p4 = 'v4';
                o[2] = 'iv2';
                o[0] = 'iv0';
                o[1] = 'iv1';
                delete o.p1;
                delete o.p3;
                o.p1 = 'v1';
            "#}),
        TestAction::assert(r#"arrayEquals(Object.keys(o), [ "0", "1", "2", "p2", "p4", "p1" ])"#),
        TestAction::assert(
            r#"arrayEquals(Object.values(o), [ "iv0", "iv1", "iv2", "v2", "v4", "v1" ])"#,
        ),
    ]);
}

// Add these additional tests at the end of the existing tests.rs file

#[test]
fn multiple_inheritance_levels_cacheable() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Level1 {
            method1() { return 1; }
        }

        class Level2 extends Level1 {
            method2() { return 2; }
        }

        class Level3 extends Level2 {
            method3() { return 3; }
        }

        class Level4 extends Level3 {
            method4() { return 4; }
        }

        let obj = new Level4();
        
        // Access methods from different prototype levels multiple times
        let results = [];
        for (let i = 0; i < 10; i++) {
            results.push(obj.method1()); // 3 levels up
            results.push(obj.method2()); // 2 levels up
            results.push(obj.method3()); // 1 level up
            results.push(obj.method4()); // own prototype
        }
        
        // Verify all calls returned correct values
        results.every((val, idx) => val === (idx % 4) + 1)
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn builtin_prototype_methods_cacheable() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        // Array.prototype methods should be cacheable
        let arr = [1, 2, 3];
        let results = [];
        
        for (let i = 0; i < 50; i++) {
            results.push(arr.length);
        }
        
        results.length === 50 && results.every(x => x === 3)
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn mixed_own_and_inherited_properties() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Base {
            inherited() { return "inherited"; }
        }

        class Derived extends Base {
            constructor() {
                super();
                this.own = "own";
            }
            derivedMethod() { return "derived"; }
        }

        let obj = new Derived();
        let results = [];
        
        // Mix accessing own properties and inherited methods
        for (let i = 0; i < 20; i++) {
            results.push(obj.own);              // own property
            results.push(obj.derivedMethod());  // direct prototype
            results.push(obj.inherited());      // transitive prototype
        }
        
        results.length === 60 &&
        results.filter(x => x === "own").length === 20 &&
        results.filter(x => x === "derived").length === 20 &&
        results.filter(x => x === "inherited").length === 20
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn cache_invalidation_on_prototype_change() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Base {
            method() { return "original"; }
        }

        class Derived extends Base {}

        let obj = new Derived();
        
        // Call method multiple times to populate cache
        let results1 = [];
        for (let i = 0; i < 10; i++) {
            results1.push(obj.method());
        }
        
        // Modify the prototype
        Base.prototype.method = function() { return "modified"; };
        
        // Call again - cache should be invalidated
        let results2 = [];
        for (let i = 0; i < 10; i++) {
            results2.push(obj.method());
        }
        
        results1.every(x => x === "original") &&
        results2.every(x => x === "modified")
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn different_objects_same_prototype_chain() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Animal {
            speak() { return "sound"; }
        }

        class Dog extends Animal {}

        let dog1 = new Dog();
        let dog2 = new Dog();
        let dog3 = new Dog();
        
        let results = [];
        
        // Multiple objects with same prototype chain should all benefit from cache
        for (let i = 0; i < 10; i++) {
            results.push(dog1.speak());
            results.push(dog2.speak());
            results.push(dog3.speak());
        }
        
        results.length === 30 && results.every(x => x === "sound")
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn property_shadowing_with_cache() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Base {
            value() { return "base"; }
        }

        class Middle extends Base {
            value() { return "middle"; }
        }

        class Derived extends Middle {}

        let obj = new Derived();
        
        // Should get "middle", not "base" (property shadowing)
        let results = [];
        for (let i = 0; i < 20; i++) {
            results.push(obj.value());
        }
        
        results.every(x => x === "middle")
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn accessor_properties_in_prototype() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Base {
            get computed() { return this._value * 2; }
        }

        class Derived extends Base {
            constructor() {
                super();
                this._value = 21;
            }
        }

        let obj = new Derived();
        
        let results = [];
        for (let i = 0; i < 15; i++) {
            results.push(obj.computed);
        }
        
        results.every(x => x === 42)
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn very_deep_prototype_chain() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        // Create a 10-level deep prototype chain
        class L0 { m0() { return 0; } }
        class L1 extends L0 { m1() { return 1; } }
        class L2 extends L1 { m2() { return 2; } }
        class L3 extends L2 { m3() { return 3; } }
        class L4 extends L3 { m4() { return 4; } }
        class L5 extends L4 { m5() { return 5; } }
        class L6 extends L5 { m6() { return 6; } }
        class L7 extends L6 { m7() { return 7; } }
        class L8 extends L7 { m8() { return 8; } }
        class L9 extends L8 { m9() { return 9; } }

        let obj = new L9();
        
        // Access method from the deepest level multiple times
        let results = [];
        for (let i = 0; i < 25; i++) {
            results.push(obj.m0()); // 9 levels up!
        }
        
        results.every(x => x === 0)
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn prototype_property_with_undefined_value() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Base {
            getUndefined() { return undefined; }
        }

        class Derived extends Base {}

        let obj = new Derived();
        
        let results = [];
        for (let i = 0; i < 10; i++) {
            results.push(obj.getUndefined());
        }
        
        results.every(x => x === undefined)
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn stress_test_many_iterations() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class GrandParent {
            method() { return 42; }
        }

        class Parent extends GrandParent {}
        class Child extends Parent {}

        let obj = new Child();
        
        let sum = 0;
        // 1000 iterations to stress test the cache
        for (let i = 0; i < 1000; i++) {
            sum += obj.method();
        }
        
        sum === 42000
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn multiple_properties_same_prototype() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Base {
            methodA() { return "A"; }
            methodB() { return "B"; }
            methodC() { return "C"; }
        }

        class Derived extends Base {}

        let obj = new Derived();
        
        let results = [];
        for (let i = 0; i < 15; i++) {
            results.push(obj.methodA());
            results.push(obj.methodB());
            results.push(obj.methodC());
        }
        
        results.filter(x => x === "A").length === 15 &&
        results.filter(x => x === "B").length === 15 &&
        results.filter(x => x === "C").length === 15
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn constructor_property_inheritance() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class Base {}
        class Derived extends Base {}

        let obj = new Derived();
        
        // Access constructor property multiple times (it's in the prototype)
        let results = [];
        for (let i = 0; i < 20; i++) {
            results.push(obj.constructor === Derived);
        }
        
        results.every(x => x === true)
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn get_transitive_prototype_property_cacheable() {
    use crate::{
        Context, js_string,
        object::{ObjectInitializer, internal_methods::InternalMethodPropertyContext},
        property::{Attribute, PropertyKey},
    };

    let context = &mut Context::default();

    // Create X.prototype with propX
    let x_proto = ObjectInitializer::new(context)
        .property(js_string!("propX"), 42, Attribute::all())
        .build();

    // Create Y.prototype that inherits from X.prototype
    let y_proto = ObjectInitializer::with_native_data_and_proto((), x_proto.clone(), context)
        .property(js_string!("propY"), 100, Attribute::all())
        .build();

    // Create instance y that inherits from Y.prototype
    let y_instance =
        ObjectInitializer::with_native_data_and_proto((), y_proto.clone(), context).build();

    let property: PropertyKey = js_string!("propX").into();

    let ic_context = &mut InternalMethodPropertyContext::new(context);

    // Access propX on y_instance
    // Chain: y_instance -> Y.prototype -> X.prototype (propX is here)
    y_instance
        .__get__(&property, y_instance.clone().into(), ic_context)
        .expect("should not fail");

    assert!(
        ic_context.slot().in_prototype(),
        "Property should be marked as in prototype"
    );

    assert!(
        ic_context.slot().is_cacheable(),
        "Transitive prototype property MUST be cacheable"
    );

    // Verify the property is found in X.prototype
    let x_proto_shape = x_proto.borrow().shape().clone();
    let slot = x_proto_shape.lookup(&property);

    assert!(slot.is_some(), "Property should be found in X.prototype");

    let found_slot = slot.expect("Property should be found");
    assert_eq!(ic_context.slot().index, found_slot.index);
}

#[test]
fn transitive_prototype_caching_with_classes() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class X {
            getX() {
                return 42;
            }
        }

        class Y extends X {
            getY() {
                return 100;
            }
        }

        let y = new Y();
        
        // First call to getX - cache miss, will populate cache
        let result1 = y.getX();
        
        // Second call to getX - should hit the transitive prototype cache
        let result2 = y.getX();
        
        // Verify both calls work correctly
        result1 === 42 && result2 === 42
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}

#[test]
fn deep_prototype_chain_cacheable() {
    use crate::{Context, JsValue, Source};

    let context = &mut Context::default();

    let script = r#"
        class A {
            methodA() { return 1; }
        }

        class B extends A {
            methodB() { return 2; }
        }

        class C extends B {
            methodC() { return 3; }
        }

        let c = new C();
        
        // All methods should be cacheable, even methodA which is 2 levels up
        c.methodA() === 1 && c.methodB() === 2 && c.methodC() === 3
    "#;

    let result = context
        .eval(Source::from_bytes(script))
        .expect("script should execute");
    assert_eq!(result, JsValue::from(true));
}
