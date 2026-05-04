use rjsi::{DefaultRuntime, FromJs, IntoJs, Runtime, ToJs};

#[derive(Debug, IntoJs, FromJs)]
struct Person {
    name: String,
    age: i32,
}

#[derive(Debug, IntoJs, FromJs)]
struct Point(f64, f64);

fn main() {
    let mut runtime = DefaultRuntime::default();

    let (greeting, point_sum, point_from_js, person_from_js) = runtime.with_scope(|cx| {
        let person = Person {
            name: "Ada".to_string(),
            age: 36,
        };
        let person_value = person.to_js(cx).unwrap();
        let globals = cx.globals();
        globals.set(cx, "person", person_value).unwrap();

        let greeting_value = cx.eval("person.name + ' is ' + person.age").unwrap();
        let greeting = greeting_value.to_string(cx).unwrap();

        let point = Point(1.5, 2.5);
        let point_value = point.to_js(cx).unwrap();
        globals.set(cx, "point", point_value).unwrap();

        let point_sum_value = cx.eval("point[0] + point[1]").unwrap();
        let point_sum = point_sum_value.to_f64(cx).unwrap();

        let point_from_js_value = cx.eval("[3, 4]").unwrap();
        let point_from_js = Point::from_js(cx, point_from_js_value).unwrap();

        let person_from_js_value = cx.eval("({ name: 'Grace', age: 35 })").unwrap();
        let person_from_js = Person::from_js(cx, person_from_js_value).unwrap();

        (greeting, point_sum, point_from_js, person_from_js)
    });

    println!("greeting: {greeting}");
    println!("point sum from rust: {point_sum}");
    println!("point from js: {point_from_js:?}");
    println!("person from js: {person_from_js:?}");
}
