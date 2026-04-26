use rjsi::quickjs::QuickJsRuntime;
use rjsi::quickjs::{QuickJsError, QuickJsRuntimeContext};

use rjsi::bind;
use rjsi::{ContextLike, ScopeLike, ValueLike, FromJs, IntoJs};

#[derive(Debug, FromJs, IntoJs, PartialEq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

fn main() -> Result<(), QuickJsError> {
    let runtime = QuickJsRuntimeContext::new();

    runtime.with_scope(|scope| {
        let make = scope.function(bind(
            |_scope, (a, b): (i32, i32)| Ok::<_, QuickJsError>(Point { x: a, y: b }),
        ))?;

        let global = scope.global();
        global.set(scope, "makePoint", make);

        let v = scope.eval("makePoint(3, 4);")?;
        let p = <Point as FromJs<'_, QuickJsRuntime>>::from_js(scope, v)?;
        assert_eq!(p, Point { x: 3, y: 4 });
        println!("makePoint(3, 4) => {p:?}");

        Ok(())
    })
}
