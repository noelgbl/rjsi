use rjsi::DefaultEngine;
use rjsi::futures::AsyncRuntime;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let rt = AsyncRuntime::<DefaultEngine>::new();
    let handle = rt.handle();
    let engine_name = rt.engine_name();

    let promise = rt
        .with_scope(|cx| {
            let v = cx.eval("Promise.resolve(42)")?;
            Ok(cx.persist_value(v))
        })
        .await
        .unwrap();

    let result = handle.await_promise(promise).await.unwrap();
    let n = match result {
        Ok(p) => handle.with_scope(|cx| p.restore(cx).unwrap().to_f64(cx).unwrap()),
        Err(_) => panic!("Promise rejected"),
    };
    println!("[{engine_name}] awaited Promise resolved to {n}");
}
