use std::time::Duration;

use rjsi::futures::{AsyncRuntime, ContextAsyncExt};
use rjsi::{DefaultEngine, console};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let rt = AsyncRuntime::<DefaultEngine>::new();
    let handle = rt.handle();

    rt.with_scope(|cx| {
        console::init(cx)?;

        let ms_arg_idx = 0;
        let delay_fn = cx.async_function(&handle, "delay", move |handle, args| async move {
            let ms = handle.with_scope(|cx| -> rjsi::Result<u64> {
                let v = args
                    .get(ms_arg_idx)
                    .expect("delay(ms) requires an argument");
                let val = v.restore(cx)?;
                Ok(val.try_as_f64(cx)? as u64)
            })?;
            tokio::time::sleep(Duration::from_millis(ms)).await;
            Ok(())
        })?;

        let globals = cx.globals();
        globals.set(cx, "delay", delay_fn.into_value())?;

        cx.eval(&format!(
            "console.log('Starting');
             delay(50)
                 .then(() => {{ console.log('Resolved after 50ms'); }})
                 .catch((e) => {{ console.log('Error:', e); }});"
        ))?;
        Ok(())
    })
    .await
    .unwrap();

    rt.idle().await.unwrap();
}
