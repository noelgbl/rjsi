use super::handle::RuntimeHandle;
use crate::capabilities::{Microtasks, PromiseState, Promises};
use crate::context::ContextPromiseExt;
use crate::{Engine, PersistentValue, Result};

impl<E: Engine + Promises + Microtasks> RuntimeHandle<E> {
    pub async fn await_promise(
        &self,
        promise: PersistentValue<E>,
    ) -> Result<std::result::Result<PersistentValue<E>, PersistentValue<E>>> {
        loop {
            let settled = self.with_scope_and_pump(|cx| {
                let restored = promise.restore(cx)?;
                let obj = restored.try_as_object()?;
                match cx.promise_state(&obj)? {
                    PromiseState::Pending => Ok(None),
                    _ => {
                        let r = cx
                            .promise_result(&obj)?
                            .expect("settled promise must have a result");
                        Ok(Some(match r {
                            Ok(v) => Ok(cx.persist_value(v)),
                            Err(e) => Err(cx.persist_value(e)),
                        }))
                    }
                }
            })?;

            if let Some(s) = settled {
                return Ok(s);
            }
            tokio::task::yield_now().await;
        }
    }
}
