use rjsi::{ContextClassExt, DefaultRuntime, Runtime, js_methods};

struct Counter {
    value: i32,
    step: i32,
}

#[js_methods]
impl Counter {
    #[js_constructor]
    fn new(initial: i32, step: i32) -> Self {
        Self {
            value: initial,
            step,
        }
    }

    fn increment(&mut self) {
        self.value += self.step;
    }

    fn decrement(&mut self) {
        self.value -= self.step;
    }

    fn get_value(&self) -> i32 {
        self.value
    }

    fn reset(&mut self) {
        self.value = 0;
    }
}

fn main() {
    let mut runtime = DefaultRuntime::default();

    let results = runtime.with_scope(|cx| {
        // Register the class and expose it as a global.
        let ctor = cx.register_class::<Counter>().unwrap();
        cx.globals().set(cx, "Counter", ctor.into_value()).unwrap();

        // Create instances and call methods from JS.
        let result = cx
            .eval(
                r"
                const c = new Counter(10, 3);
                c.increment();
                c.increment();
                const after_two_increments = c.getValue();

                c.decrement();
                const after_decrement = c.getValue();

                c.reset();
                const after_reset = c.getValue();

                [after_two_increments, after_decrement, after_reset]
            ",
            )
            .unwrap();

        let arr = result.try_as_object().unwrap();

        let a = arr.get(cx, 0u32).unwrap().to_f64(cx).unwrap() as i32;
        let b = arr.get(cx, 1u32).unwrap().to_f64(cx).unwrap() as i32;
        let c = arr.get(cx, 2u32).unwrap().to_f64(cx).unwrap() as i32;

        (a, b, c)
    });

    println!("after two increments: {}", results.0);
    println!("after one decrement: {}", results.1);
    println!("after reset: {}", results.2);

    assert_eq!(results.0, 16);
    assert_eq!(results.1, 13);
    assert_eq!(results.2, 0);
}
